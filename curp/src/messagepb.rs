/// Propose command from client to servers
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProposeRequest {
    /// The serialized command
    /// Original type is Command trait
    #[prost(bytes="vec", tag="1")]
    pub command: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProposeResponse {
    #[prost(string, optional, tag="1")]
    pub leader_id: ::core::option::Option<::prost::alloc::string::String>,
    #[prost(uint64, tag="2")]
    pub term: u64,
    #[prost(oneof="propose_response::ExeResult", tags="3, 4")]
    pub exe_result: ::core::option::Option<propose_response::ExeResult>,
}
/// Nested message and enum types in `ProposeResponse`.
pub mod propose_response {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum ExeResult {
        /// The original type is Command::ER
        #[prost(bytes, tag="3")]
        Result(::prost::alloc::vec::Vec<u8>),
        /// The original type is ProposeError
        #[prost(bytes, tag="4")]
        Error(::prost::alloc::vec::Vec<u8>),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FetchLeaderRequest {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FetchLeaderResponse {
    #[prost(string, optional, tag="1")]
    pub leader_id: ::core::option::Option<::prost::alloc::string::String>,
    #[prost(uint64, tag="2")]
    pub term: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WaitSyncedRequest {
    #[prost(bytes="vec", tag="1")]
    pub id: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WaitSyncedResponse {
    #[prost(oneof="wait_synced_response::SyncResult", tags="1, 2")]
    pub sync_result: ::core::option::Option<wait_synced_response::SyncResult>,
}
/// Nested message and enum types in `WaitSyncedResponse`.
pub mod wait_synced_response {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Success {
        #[prost(bytes="vec", tag="1")]
        pub after_sync_result: ::prost::alloc::vec::Vec<u8>,
        #[prost(bytes="vec", tag="2")]
        pub exe_result: ::prost::alloc::vec::Vec<u8>,
    }
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum SyncResult {
        #[prost(message, tag="1")]
        Success(Success),
        #[prost(bytes, tag="2")]
        Error(::prost::alloc::vec::Vec<u8>),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AppendEntriesRequest {
    #[prost(uint64, tag="1")]
    pub term: u64,
    #[prost(string, tag="2")]
    pub leader_id: ::prost::alloc::string::String,
    #[prost(uint64, tag="3")]
    pub prev_log_index: u64,
    #[prost(uint64, tag="4")]
    pub prev_log_term: u64,
    #[prost(bytes="vec", repeated, tag="5")]
    pub entries: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    #[prost(uint64, tag="6")]
    pub leader_commit: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AppendEntriesResponse {
    #[prost(uint64, tag="1")]
    pub term: u64,
    #[prost(bool, tag="2")]
    pub success: bool,
    #[prost(uint64, tag="3")]
    pub commit_index: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct VoteRequest {
    #[prost(uint64, tag="1")]
    pub term: u64,
    #[prost(string, tag="2")]
    pub candidate_id: ::prost::alloc::string::String,
    #[prost(uint64, tag="3")]
    pub last_log_index: u64,
    #[prost(uint64, tag="4")]
    pub last_log_term: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct VoteResponse {
    #[prost(uint64, tag="1")]
    pub term: u64,
    #[prost(bool, tag="2")]
    pub vote_granted: bool,
}
/// Generated client implementations.
pub mod protocol_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    #[derive(Debug, Clone)]
    pub struct ProtocolClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl ProtocolClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: std::convert::TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> ProtocolClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> ProtocolClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
            >>::Error: Into<StdError> + Send + Sync,
        {
            ProtocolClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with `gzip`.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_gzip(mut self) -> Self {
            self.inner = self.inner.send_gzip();
            self
        }
        /// Enable decompressing responses with `gzip`.
        #[must_use]
        pub fn accept_gzip(mut self) -> Self {
            self.inner = self.inner.accept_gzip();
            self
        }
        pub async fn propose(
            &mut self,
            request: impl tonic::IntoRequest<super::ProposeRequest>,
        ) -> Result<tonic::Response<super::ProposeResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/messagepb.Protocol/Propose",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn wait_synced(
            &mut self,
            request: impl tonic::IntoRequest<super::WaitSyncedRequest>,
        ) -> Result<tonic::Response<super::WaitSyncedResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/messagepb.Protocol/WaitSynced",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn append_entries(
            &mut self,
            request: impl tonic::IntoRequest<super::AppendEntriesRequest>,
        ) -> Result<tonic::Response<super::AppendEntriesResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/messagepb.Protocol/AppendEntries",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn vote(
            &mut self,
            request: impl tonic::IntoRequest<super::VoteRequest>,
        ) -> Result<tonic::Response<super::VoteResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/messagepb.Protocol/Vote");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn fetch_leader(
            &mut self,
            request: impl tonic::IntoRequest<super::FetchLeaderRequest>,
        ) -> Result<tonic::Response<super::FetchLeaderResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/messagepb.Protocol/FetchLeader",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
    }
}
/// Generated server implementations.
pub mod protocol_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    ///Generated trait containing gRPC methods that should be implemented for use with ProtocolServer.
    #[async_trait]
    pub trait Protocol: Send + Sync + 'static {
        async fn propose(
            &self,
            request: tonic::Request<super::ProposeRequest>,
        ) -> Result<tonic::Response<super::ProposeResponse>, tonic::Status>;
        async fn wait_synced(
            &self,
            request: tonic::Request<super::WaitSyncedRequest>,
        ) -> Result<tonic::Response<super::WaitSyncedResponse>, tonic::Status>;
        async fn append_entries(
            &self,
            request: tonic::Request<super::AppendEntriesRequest>,
        ) -> Result<tonic::Response<super::AppendEntriesResponse>, tonic::Status>;
        async fn vote(
            &self,
            request: tonic::Request<super::VoteRequest>,
        ) -> Result<tonic::Response<super::VoteResponse>, tonic::Status>;
        async fn fetch_leader(
            &self,
            request: tonic::Request<super::FetchLeaderRequest>,
        ) -> Result<tonic::Response<super::FetchLeaderResponse>, tonic::Status>;
    }
    #[derive(Debug)]
    pub struct ProtocolServer<T: Protocol> {
        inner: _Inner<T>,
        accept_compression_encodings: (),
        send_compression_encodings: (),
    }
    struct _Inner<T>(Arc<T>);
    impl<T: Protocol> ProtocolServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for ProtocolServer<T>
    where
        T: Protocol,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/messagepb.Protocol/Propose" => {
                    #[allow(non_camel_case_types)]
                    struct ProposeSvc<T: Protocol>(pub Arc<T>);
                    impl<T: Protocol> tonic::server::UnaryService<super::ProposeRequest>
                    for ProposeSvc<T> {
                        type Response = super::ProposeResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ProposeRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).propose(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ProposeSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/messagepb.Protocol/WaitSynced" => {
                    #[allow(non_camel_case_types)]
                    struct WaitSyncedSvc<T: Protocol>(pub Arc<T>);
                    impl<
                        T: Protocol,
                    > tonic::server::UnaryService<super::WaitSyncedRequest>
                    for WaitSyncedSvc<T> {
                        type Response = super::WaitSyncedResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::WaitSyncedRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).wait_synced(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = WaitSyncedSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/messagepb.Protocol/AppendEntries" => {
                    #[allow(non_camel_case_types)]
                    struct AppendEntriesSvc<T: Protocol>(pub Arc<T>);
                    impl<
                        T: Protocol,
                    > tonic::server::UnaryService<super::AppendEntriesRequest>
                    for AppendEntriesSvc<T> {
                        type Response = super::AppendEntriesResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::AppendEntriesRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).append_entries(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = AppendEntriesSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/messagepb.Protocol/Vote" => {
                    #[allow(non_camel_case_types)]
                    struct VoteSvc<T: Protocol>(pub Arc<T>);
                    impl<T: Protocol> tonic::server::UnaryService<super::VoteRequest>
                    for VoteSvc<T> {
                        type Response = super::VoteResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::VoteRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).vote(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = VoteSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/messagepb.Protocol/FetchLeader" => {
                    #[allow(non_camel_case_types)]
                    struct FetchLeaderSvc<T: Protocol>(pub Arc<T>);
                    impl<
                        T: Protocol,
                    > tonic::server::UnaryService<super::FetchLeaderRequest>
                    for FetchLeaderSvc<T> {
                        type Response = super::FetchLeaderResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::FetchLeaderRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).fetch_leader(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = FetchLeaderSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        Ok(
                            http::Response::builder()
                                .status(200)
                                .header("grpc-status", "12")
                                .header("content-type", "application/grpc")
                                .body(empty_body())
                                .unwrap(),
                        )
                    })
                }
            }
        }
    }
    impl<T: Protocol> Clone for ProtocolServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
            }
        }
    }
    impl<T: Protocol> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: Protocol> tonic::transport::NamedService for ProtocolServer<T> {
        const NAME: &'static str = "messagepb.Protocol";
    }
}
