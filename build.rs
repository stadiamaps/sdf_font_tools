use protobuf_codegen_pure::Codegen;

fn main() {
    Codegen::new()
        .out_dir("src/")
        .input("proto/glyphs.proto")
        .include("proto")
        .run()
        .expect("Protobuf codegen failed.");
}
