#[cfg(test)]
use std::sync::atomic::AtomicBool;
use std::{iter, ops::Range, sync::Arc, time::Duration};

use clippy_utilities::NumericCast;
use futures::future::Either;
use lock_utils::parking_lot_lock::RwLockMap;
use madsim::rand::{thread_rng, Rng};
use parking_lot::{lock_api::RwLockUpgradableReadGuard, Mutex, RwLock};
use tokio::{sync::mpsc, task::JoinHandle, time::Instant};
use tracing::{debug, error, info, warn};

use super::{
    cmd_board::{CmdState, CommandBoard},
    SyncMessage,
};
use crate::{
    cmd::{Command, CommandExecutor},
    log::LogEntry,
    rpc::{self, AppendEntriesRequest, Connect, VoteRequest, WaitSyncedResponse},
    server::{
        cmd_execute_worker::{execute_worker, CmdExeReceiver, CmdExeSender, N_EXECUTE_WORKERS},
        ServerRole, SpeculativePool, State,
    },
    shutdown::Shutdown,
    LogIndex,
};

/// Wait for sometime before next retry
// TODO: make it configurable
const RETRY_TIMEOUT: Duration = Duration::from_millis(800);

/// Run background tasks
#[allow(clippy::too_many_arguments)] // we call this function once, it's ok
pub(super) async fn run_bg_tasks<C: Command + 'static, CE: 'static + CommandExecutor<C>>(
    state: Arc<RwLock<State<C>>>,
    sync_chan: flume::Receiver<SyncMessage<C>>,
    cmd_executor: CE,
    spec: Arc<Mutex<SpeculativePool<C>>>,
    cmd_exe_tx: CmdExeSender<C>,
    cmd_exe_rx: CmdExeReceiver<C>,
    mut shutdown: Shutdown,
    #[cfg(test)] reachable: Arc<AtomicBool>,
) {
    // establish connection with other servers
    let others = state.read().others.clone();

    // 创建到 others 的 RPC Clients
    let connects = rpc::try_connect(
        others,
        #[cfg(test)]
        reachable,
    )
    .await;

    // notify when a broadcast of append_entries is needed immediately
    let (ae_trigger, ae_trigger_rx) = mpsc::unbounded_channel::<usize>();

    // 后台任务
    //  0.
    let bg_ae_handle = tokio::spawn(bg_append_entries(
        connects.clone(),
        Arc::clone(&state),
        ae_trigger_rx,
    ));
    //  1. bg_election: 选举线程
    let bg_election_handle = tokio::spawn(bg_election(connects.clone(), Arc::clone(&state)));

    //  2.
    let bg_apply_handle = tokio::spawn(bg_apply(Arc::clone(&state), cmd_exe_tx, spec));

    //  3. bg_heartbeat: 心跳线程
    let bg_heartbeat_handle = tokio::spawn(bg_heartbeat(connects.clone(), Arc::clone(&state)));

    //  4. 同步 (term, cmd)。从 sync_chan 读取 (term, cmd) 写入到本地的 log，并将最新的 log index 写入到 ae_trigger
    //  sync_chan 的生产者链路
    //  -> curp::rpc::Rpc::rpc::propose
    //      -> curp::server::Protocol::propose
    //          -> curp::server::Protocol::sync_to_others
    //  调用 propose 时，会写入一条消息到 sync_chan
    let bg_get_sync_cmds_handle =
        tokio::spawn(bg_get_sync_cmds(Arc::clone(&state), sync_chan, ae_trigger));

    //  5.
    let calibrate_handle = tokio::spawn(leader_calibrates_followers(connects, state));

    // spawn cmd execute worker
    let bg_exe_worker_handles: Vec<JoinHandle<_>> =
        iter::repeat((cmd_exe_rx, Arc::new(cmd_executor)))
            .take(N_EXECUTE_WORKERS)
            .map(|(rx, ce)| tokio::spawn(execute_worker(rx, ce)))
            .collect();

    shutdown.recv().await;
    bg_ae_handle.abort();
    bg_election_handle.abort();
    bg_apply_handle.abort();
    bg_heartbeat_handle.abort();
    bg_get_sync_cmds_handle.abort();
    calibrate_handle.abort();
    for handle in bg_exe_worker_handles {
        handle.abort();
    }
    info!("all background task stopped");
}

/// Fetch commands need to be synced and add them to the log
async fn bg_get_sync_cmds<C: Command + 'static>(
    state: Arc<RwLock<State<C>>>,
    sync_chan: flume::Receiver<SyncMessage<C>>,
    ae_trigger: mpsc::UnboundedSender<usize>,
) {
    loop {
        let (term, cmd) = match sync_chan.recv_async().await {
            Ok(msg) => msg.inner(),
            Err(_) => {
                return;
            }
        };

        #[allow(clippy::shadow_unrelated)] // clippy false positive
        state.map_write(|mut state| {
            // 将 (term, cmd) 写入到本地的 log，并将 log.len() - 1 发送到 ae_trigger(append_entries_trigger)
            //
            // 注意，因为:
            //  1. log[0] 是一个 fake log，因此 log.len() >= 1 恒成立
            //  2. 当插入 n 条日志时，log.len() = n + 1
            //  3. 而 state.last_log_index() = log.len() - 1
            // 因此:
            //  state.last_log_index() = n + 1 - 1 = n
            //
            // 假设这个收到了第一个 (term, cmd)，原 log 数组长度为 1，则该条数据的索引为 1，log.len() = 2
            // 此时 state.last_log_index() = log.len() - 1 = 2 - 2 = 1
            // 也就是会将 1 写入到 ae_trigger
            state.log.push(LogEntry::new(term, &[cmd]));
            if let Err(e) = ae_trigger.send(state.last_log_index()) {
                error!("ae_trigger failed: {}", e);
            }

            debug!(
                "received new log, index {:?}",
                state.log.len().checked_sub(1),
            );
        });
    }
}

/// Interval between sending heartbeats
const HEARTBEAT_INTERVAL: Duration = Duration::from_millis(150);
/// Rpc request timeout
const RPC_TIMEOUT: Duration = Duration::from_millis(50);

/// Background `append_entries`, only works for the leader
///
/// LEADER 将 log 发送到 peers
async fn bg_append_entries<C: Command + 'static>(
    connects: Vec<Arc<Connect>>,
    state: Arc<RwLock<State<C>>>,
    mut ae_trigger_rx: mpsc::UnboundedReceiver<usize>,
) {
    // ae_trigger_rx 对应的生产者位于: curp::server::bg_get_sync_cmds
    // 当写入第一条消息时，log 数组内有两条 LogEntry:
    //  index(0): fake log
    //  index(1): 实际写入的 log
    // 此时，ae_trigger_rx 读取到的内容为 1，即 i = 1、
    //
    // 因为，push log entry 到 state.log 时使用读写锁同步，因此，可以分析出
    // 1. 从 ae_trigger_rx 读取到的数据 i 从 1 开始，并按顺序依次增加 1
    // 2. i 表示 state.log 数组的索引
    while let Some(i) = ae_trigger_rx.recv().await {
        let req = {
            let state = state.read();
            if !state.is_leader() {
                warn!("Non leader receives sync log[{i}] request");
                continue;
            }

            // log.len() >= 1 because we have a fake log[0]
            #[allow(clippy::integer_arithmetic, clippy::indexing_slicing)]
            match AppendEntriesRequest::new(
                // 本地 term
                state.term,
                // 本地 id
                state.id().clone(),
                // 上一条数据的索引，应该是用于其他 node 定位本条日志的位置
                i - 1,
                // 上一条索引的 term
                state.log[i - 1].term(),
                // 日志数据
                vec![state.log[i].clone()],
                state.commit_index,
            ) {
                Err(e) => {
                    error!("unable to serialize append entries request: {}", e);
                    continue;
                }
                Ok(req) => req,
            }
        };

        // send append_entries to each server in parallel
        for connect in &connects {
            let _handle = tokio::spawn(send_log_until_succeed(
                i,
                req.clone(),
                Arc::clone(connect),
                Arc::clone(&state),
            ));
        }
    }
}

/// Send `append_entries` containing a single log to a server
#[allow(clippy::integer_arithmetic, clippy::indexing_slicing)] // log.len() >= 1 because we have a fake log[0], indexing of `next_index` or `match_index` won't panic because we created an entry when initializing the server state
async fn send_log_until_succeed<C: Command + 'static>(
    i: usize,
    req: AppendEntriesRequest,
    connect: Arc<Connect>,
    state: Arc<RwLock<State<C>>>,
) {
    // send log[i] until succeed
    loop {
        debug!("append_entries sent to {}", connect.id());
        let resp = connect.append_entries(req.clone(), RPC_TIMEOUT).await;

        #[allow(clippy::unwrap_used)]
        // indexing of `next_index` or `match_index` won't panic because we created an entry when initializing the server state
        match resp {
            Err(e) => {
                warn!("append_entries error: {e}");
                // wait for some time until next retry
                tokio::time::sleep(RETRY_TIMEOUT).await;
            }
            Ok(resp) => {
                let resp = resp.into_inner();
                let state = state.upgradable_read();

                // calibrate term
                if resp.term > state.term {
                    let mut state = RwLockUpgradableReadGuard::upgrade(state);
                    state.update_to_term(resp.term);
                    return;
                }

                if resp.success {
                    let mut state = RwLockUpgradableReadGuard::upgrade(state);
                    // update match_index and next_index
                    let match_index = state.match_index.get_mut(connect.id()).unwrap();
                    if *match_index < i {
                        *match_index = i;
                    }
                    *state.next_index.get_mut(connect.id()).unwrap() = *match_index + 1;

                    let min_replicated = (state.others.len() + 1) / 2;
                    // If the majority of servers has replicated the log, commit
                    if state.commit_index < i
                        && state.log[i].term() == state.term
                        && state
                            .others
                            .iter()
                            .filter(|&(id, _)| state.match_index[id] >= i)
                            .count()
                            >= min_replicated
                    {
                        state.commit_index = i;
                        debug!("commit_index updated to {i}");
                        state.commit_trigger.notify(1);
                    }
                    break;
                }
            }
        }
    }
}

/// Background `append_entries`, only works for the leader
///
/// 后台心跳线程，只有 LEADER 需要发送心跳
async fn bg_heartbeat<C: Command + 'static>(
    connects: Vec<Arc<Connect>>,
    state: Arc<RwLock<State<C>>>,
) {
    let role_trigger = state.read().role_trigger();
    #[allow(clippy::integer_arithmetic)] // tokio internal triggered
    loop {
        // only leader should run this task

        // 获取当前状态，如果不是 LEADER 则等待
        while !state.read().is_leader() {
            role_trigger.listen().await;
        }

        // 心跳间隔 150ms
        tokio::time::sleep(HEARTBEAT_INTERVAL).await;

        // send append_entries to each server in parallel
        for connect in &connects {
            let _handle = tokio::spawn(send_heartbeat(Arc::clone(connect), Arc::clone(&state)));
        }
    }
}

/// Send `append_entries` to a server
#[allow(clippy::integer_arithmetic, clippy::indexing_slicing)] // log.len() >= 1 because we have a fake log[0], indexing of `next_index` or `match_index` won't panic because we created an entry when initializing the server state
async fn send_heartbeat<C: Command + 'static>(connect: Arc<Connect>, state: Arc<RwLock<State<C>>>) {
    // prepare append_entries request args
    #[allow(clippy::shadow_unrelated)] // clippy false positive
    let req = state.map_read(|state| {
        // state.next_index 记录每个节点的 log entry 索引
        let next_index = state.next_index[connect.id()];
        // 心跳包中包含:
        //  1. leader 的 term
        //  2. leader 的 id
        //  3. 上次的日志 index
        //  4. leader 完成 commit 的 id
        AppendEntriesRequest::new_heartbeat(
            state.term,
            state.id().clone(),
            next_index - 1,
            state.log[next_index - 1].term(),
            state.commit_index,
        )
    });

    // send append_entries request and receive response
    debug!("heartbeat sent to {}", connect.id());
    let resp = connect.append_entries(req, RPC_TIMEOUT).await;

    #[allow(clippy::unwrap_used)]
    // indexing of `next_index` or `match_index` won't panic because we created an entry when initializing the server state
    match resp {
        Err(e) => warn!("append_entries error: {}", e),
        Ok(resp) => {
            let resp = resp.into_inner();
            // calibrate term
            let state = state.upgradable_read();
            if resp.term > state.term {
                let mut state = RwLockUpgradableReadGuard::upgrade(state);
                state.update_to_term(resp.term);
                return;
            }
            if !resp.success {
                let mut state = RwLockUpgradableReadGuard::upgrade(state);
                *state.next_index.get_mut(connect.id()).unwrap() -= 1;
            }
        }
    };
}

/// Background apply
async fn bg_apply<C: Command + 'static>(
    state: Arc<RwLock<State<C>>>,
    exe_tx: CmdExeSender<C>,
    spec: Arc<Mutex<SpeculativePool<C>>>,
) {
    #[allow(clippy::shadow_unrelated)]
    let (commit_trigger, cmd_board) =
        state.map_read(|state| (state.commit_trigger(), state.cmd_board()));
    loop {
        // wait until there is something to commit
        let state = loop {
            {
                let state = state.upgradable_read();
                if state.need_commit() {
                    break state;
                }
            }
            commit_trigger.listen().await;
        };

        let mut state = RwLockUpgradableReadGuard::upgrade(state);
        #[allow(clippy::integer_arithmetic, clippy::indexing_slicing)]
        // TODO: overflow of log index should be prevented
        for i in (state.last_applied + 1)..=state.commit_index {
            for cmd in state.log[i].cmds().iter() {
                let cmd_id = cmd.id();
                if state.is_leader() {
                    handle_after_sync_leader(
                        Arc::clone(&cmd_board),
                        Arc::clone(cmd),
                        i.numeric_cast(),
                        &exe_tx,
                    );
                } else {
                    handle_after_sync_follower(&exe_tx, Arc::clone(cmd), i.numeric_cast());
                }
                spec.lock().mark_ready(cmd_id);
            }
            state.last_applied = i;
            debug!("log[{i}] committed, last_applied updated to {}", i);
        }
    }
}

/// The leader handles after sync
fn handle_after_sync_leader<C: Command + 'static>(
    cmd_board: Arc<Mutex<CommandBoard>>,
    cmd: Arc<C>,
    index: LogIndex,
    exe_tx: &CmdExeSender<C>,
) {
    let cmd_id = cmd.id().clone();

    let needs_execute = {
        // the leader will see if the command needs execution from cmd board
        let cmd_board = cmd_board.lock();
        let Some(cmd_state) =   cmd_board.cmd_states.get(cmd.id()) else {
            error!("No cmd {:?} in command board", cmd.id());
            return
        };
        // let cmd_state = if let Some(cmd_state) = cmd_board.cmd_states.get(cmd.id()) {
        //     cmd_state
        // } else {
        //     error!("No cmd {:?} in command board", cmd.id());
        //     return;
        // };
        match *cmd_state {
            CmdState::Execute => true,
            CmdState::AfterSync => false,
            CmdState::EarlyArrive | CmdState::FinalResponse(_) => {
                error!("should not get state {:?} before after sync", cmd_state);
                return;
            }
        }
    };

    let resp = if needs_execute {
        let result = exe_tx.send_exe_and_after_sync(cmd, index);
        Either::Left(async move {
            result.await.map_or_else(
                |e| {
                    error!("can't receive exe result from exe worker, {e}");
                    WaitSyncedResponse::new_from_result::<C>(None, None)
                },
                |(er, asr)| WaitSyncedResponse::new_from_result::<C>(Some(er), asr),
            )
        })
    } else {
        let result = exe_tx.send_after_sync(cmd, index);
        Either::Right(async move {
            result.await.map_or_else(
                |e| {
                    error!("can't receive exe result from exe worker, {e}");
                    WaitSyncedResponse::new_from_result::<C>(None, None)
                },
                |asr| WaitSyncedResponse::new_from_result::<C>(None, Some(asr)),
            )
        })
    };

    // update the cmd_board after execution and after_sync is completed
    let _ignored = tokio::spawn(async move {
        let resp = resp.await;

        let mut cmd_board = cmd_board.lock();
        let Some(cmd_state) = cmd_board.cmd_states.get_mut(&cmd_id) else {
            error!("No cmd {:?} in command board", cmd_id);
            return;
        };
        *cmd_state = CmdState::FinalResponse(resp);

        // now we can notify the waiting request
        if let Some(notify) = cmd_board.notifiers.get(&cmd_id) {
            notify.notify(usize::MAX);
        }
    });
}

/// The follower handles after sync
fn handle_after_sync_follower<C: Command + 'static>(
    exe_tx: &CmdExeSender<C>,
    cmd: Arc<C>,
    index: LogIndex,
) {
    // FIXME: should follower store er and asr in case it becomes leader later?
    let _ignore = exe_tx.send_exe_and_after_sync(cmd, index);
}

/// How long a candidate should wait before it starts another round of election
const CANDIDATE_TIMEOUT: Duration = Duration::from_secs(1);
/// How long a follower should wait before it starts a round of election (in millis)
const FOLLOWER_TIMEOUT: Range<u64> = 1000..2000;

/// Background election
///
/// 后台选举线程
async fn bg_election<C: Command + 'static>(
    connects: Vec<Arc<Connect>>,
    state: Arc<RwLock<State<C>>>,
) {
    #[allow(clippy::shadow_unrelated)]
    let (role_trigger, last_rpc_time) =
        state.map_read(|state| (state.role_trigger(), state.last_rpc_time()));
    loop {
        // only follower or candidate should run this task
        //
        // leader 线程等待 role_trigger 的通知
        while state.read().is_leader() {
            role_trigger.listen().await;
        }

        let current_role = state.read().role();
        let start_vote = match current_role {
            ServerRole::Follower => {
                // FOLLOWER 的超时时间为 1-2s 的随机时间，要求每 1-2s 之间必须更新一次 rpc 时间(last_rpc_time)
                // 生成随机休眠时间 timeout
                let timeout = Duration::from_millis(thread_rng().gen_range(FOLLOWER_TIMEOUT));
                // wait until it needs to vote
                loop {
                    let next_check = last_rpc_time.read().to_owned() + timeout;
                    tokio::time::sleep_until(next_check).await;
                    // 这里的目的是判断，每 timeout 时间片内，至少更新过一次 last_rpc_time
                    //
                    // 原因如下:
                    //  这里的 last_rpc_time 时间可能与上方的 last_rpc_time 不同
                    //  如果 last_rpc_time 没有被更新过，也就是用于计算 next_check 的时间
                    //  那么这里由于时间精度问题，大概率当前时间已经经过了 timeout 时间
                    //
                    // 如果在 timeout 时间片内，没有更新过 last_rpc_time，则结束 loop 循环
                    // 并在赋值 start_vote = true
                    if Instant::now() - *last_rpc_time.read() > timeout {
                        break;
                    }
                }
                true
            }
            ServerRole::Candidate => loop {
                // Candidate 的超时时间为固定的 1s
                let next_check = last_rpc_time.read().to_owned() + CANDIDATE_TIMEOUT;
                tokio::time::sleep_until(next_check).await;
                // check election status

                // 到达超时时间后校验选举结果
                //
                // 状态为 Follower | Leader 时，不执行选举流程
                //  Follower: 选举失败，降级
                //  Leader:   选举成功，升级
                match state.read().role() {
                    // election failed, becomes a follower || election succeeded, becomes a leader
                    ServerRole::Follower | ServerRole::Leader => {
                        break false;
                    }
                    ServerRole::Candidate => {}
                }
                // 当满足条件时，说明自上次更新 last_rpc_time 后，没有再次更新过 last_rpc_time
                // 则进入到选举流程
                if Instant::now() - *last_rpc_time.read() > CANDIDATE_TIMEOUT {
                    break true;
                }
            },
            ServerRole::Leader => false, // leader should not vote
        };
        if !start_vote {
            continue;
        }

        // 开始新一轮选举

        // FOLLOWER:  当 timeout 时间片内没有更新过 last_rpc_time 时会进入到这里
        // CANDIDATE: 当 timeout 时间片内没有更新过 last_rpc_time 时会进入到这里

        // 20230108 的想法:
        //      首先，所有的节点应该都是 FOLLOWER 身份，由于此时不存在 Leader，则会以 FOLLOWER 身份
        //  发起选举，节点的身份变为 CANDIDATE，并发送 vote() 请求给 peers。

        // 以下为准备 VoteRequest:
        // start election
        #[allow(clippy::integer_arithmetic)] // TODO: handle possible overflow
        let req = {
            let mut state = state.write();
            // 更新 term
            let new_term = state.term + 1;
            // update_to_term 会清除之前的投票信息，并将 role 设置为 FOLLOWER
            // 因此，需要再次调用 state.set_role(ServerRole::Candidate)
            state.update_to_term(new_term);
            state.set_role(ServerRole::Candidate);
            // 发起选举，默认给自己投一票
            state.voted_for = Some(state.id().clone());
            state.votes_received = 1;
            VoteRequest::new(
                state.term,
                state.id().clone(),
                state.last_log_index(),
                state.last_log_term(),
            )
        };
        // reset
        // 发送 RPC 之前，更新 last_rpc_time
        *last_rpc_time.write() = Instant::now();
        debug!("server {} starts election", req.candidate_id);

        for connect in &connects {
            let _ignore = tokio::spawn(send_vote(
                Arc::clone(connect),
                Arc::clone(&state),
                req.clone(),
            ));
        }
    }
}

/// send vote request
async fn send_vote<C: Command + 'static>(
    connect: Arc<Connect>,
    state: Arc<RwLock<State<C>>>,
    req: VoteRequest,
) {
    // RPC Server 端位于 curp::server::Protocol::vote 函数
    let resp = connect.vote(req, RPC_TIMEOUT).await;
    match resp {
        Err(e) => error!("vote failed, {}", e),
        Ok(resp) => {
            let resp = resp.into_inner();

            // calibrate term
            let state = state.upgradable_read();
            // 如果响应的 term > 本地的 term，则更新本地 term，终止选举并清除相关数据
            if resp.term > state.term {
                let mut state = RwLockUpgradableReadGuard::upgrade(state);
                state.update_to_term(resp.term);
                return;
            }

            // is still a candidate
            if !matches!(state.role(), ServerRole::Candidate) {
                return;
            }

            #[allow(clippy::integer_arithmetic)]
            if resp.vote_granted {
                debug!("{}: vote is granted by server {}", state.id(), connect.id());
                let mut state = RwLockUpgradableReadGuard::upgrade(state);
                state.votes_received += 1;

                // the majority has granted the vote
                let min_granted = (state.others.len() + 1) / 2 + 1;
                if state.votes_received >= min_granted {
                    state.set_role(ServerRole::Leader);
                    state.leader_id = Some(state.id().clone());
                    debug!("server {} becomes leader", state.id());

                    // init next_index
                    let last_log_index = state.last_log_index();
                    for index in state.next_index.values_mut() {
                        *index = last_log_index + 1; // iter from the end to front is more likely to match the follower
                    }

                    // trigger heartbeat immediately to establish leadership
                    state.calibrate_trigger.notify(1);
                }
            }
        }
    }
}

/// Leader should first enforce followers to be consistent with it when it comes to power
#[allow(clippy::integer_arithmetic, clippy::indexing_slicing)] // log.len() >= 1 because we have a fake log[0], indexing of `next_index` or `match_index` won't panic because we created an entry when initializing the server state
async fn leader_calibrates_followers<C: Command + 'static>(
    connects: Vec<Arc<Connect>>,
    state: Arc<RwLock<State<C>>>,
) {
    let calibrate_trigger = Arc::clone(&state.read().calibrate_trigger);
    loop {
        calibrate_trigger.listen().await;
        for connect in connects.clone() {
            let state = Arc::clone(&state);
            let _handle = tokio::spawn(async move {
                loop {
                    // send append entry
                    #[allow(clippy::shadow_unrelated)] // clippy false positive
                    let (req, last_sent_index) = {
                        let state = state.read();
                        let next_index = state.next_index[connect.id()];
                        match AppendEntriesRequest::new(
                            state.term,
                            state.id().clone(),
                            next_index - 1,
                            state.log[next_index - 1].term(),
                            state.log[next_index..].to_vec(),
                            state.commit_index,
                        ) {
                            Err(e) => {
                                error!("unable to serialize append entries request: {}", e);
                                return;
                            }
                            Ok(req) => (req, state.log.len() - 1),
                        }
                    };

                    let resp = connect.append_entries(req, RPC_TIMEOUT).await;

                    #[allow(clippy::unwrap_used)]
                    // indexing of `next_index` or `match_index` won't panic because we created an entry when initializing the server state
                    match resp {
                        Err(e) => {
                            warn!("append_entries error: {}", e);
                            tokio::time::sleep(RETRY_TIMEOUT).await;
                        }
                        Ok(resp) => {
                            let resp = resp.into_inner();
                            // calibrate term
                            let state = state.upgradable_read();
                            if resp.term > state.term {
                                let mut state = RwLockUpgradableReadGuard::upgrade(state);
                                state.update_to_term(resp.term);
                                return;
                            }

                            let mut state = RwLockUpgradableReadGuard::upgrade(state);

                            // successfully calibrate
                            if resp.success {
                                *state.next_index.get_mut(connect.id()).unwrap() =
                                    last_sent_index + 1;
                                *state.match_index.get_mut(connect.id()).unwrap() = last_sent_index;
                                break;
                            }

                            *state.next_index.get_mut(connect.id()).unwrap() =
                                (resp.commit_index + 1).numeric_cast();
                        }
                    };
                }
            });
        }
    }
}
