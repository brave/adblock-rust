fn main() {
    capnpc::CompilerCommand::new()
        .src_prefix("src/capnp")
        .file("src/capnp/network_filter.capnp")
        .run()
        .expect("schema compiler command");
}
