fn main() {
    protobuf_codegen::Codegen::new()
        .cargo_out_dir("protos")
        .input("proto/glyphs.proto")
        .include("proto/")
        .run()
        .expect("Protobuf codegen failed.");
}
