fn main() {
    let protoc_path = protoc_bin_vendored::protoc_bin_path().expect("vendored protoc binary");
    std::env::set_var("PROTOC", protoc_path);

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["proto/arena.proto"], &["proto"])
        .expect("compiling arena.proto");
}
