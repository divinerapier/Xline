fn main() {
    tonic_build::configure()
        .out_dir("./src")
        .compile(&["./proto/message.proto"], &["./proto/"])
        .unwrap_or_else(|e| panic!("Failed to compile proto, error is {:?}", e));
}
