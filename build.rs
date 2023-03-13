use protoc_bin_vendored::protoc_bin_path;

fn main() {
    let mut codegen = protobuf_codegen::Codegen::new();
    codegen
        .cargo_out_dir("protos")
        .input("proto/glyphs.proto")
        .include("proto/");
    if let Ok(vendored_protoc) = protoc_bin_path() {
        codegen.protoc_path(&vendored_protoc);
    }
    codegen.run().expect("Protobuf codegen failed.");
}
