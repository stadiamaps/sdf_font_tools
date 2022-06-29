use protoc_bin_vendored::protoc_bin_path;

fn main() {
    protobuf_codegen::Codegen::new()
        .cargo_out_dir("protos")
        .protoc_path(&protoc_bin_path().unwrap())
        .input("proto/glyphs.proto")
        .include("proto/")
        .run()
        .expect("Protobuf codegen failed.");
}
