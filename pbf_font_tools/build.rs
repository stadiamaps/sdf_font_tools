use std::io::Result;

fn main() -> Result<()> {
    #[cfg(feature = "build-protobuf-src")]
    std::env::set_var("PROTOC", protobuf_src::protoc());
    prost_build::compile_protos(&["proto/glyphs.proto"], &["proto/"])?;
    Ok(())
}
