use std::io::Result;

fn main() -> Result<()> {
    #[cfg(feature = "protoc-from-src")]
    std::env::set_var("PROTOC", protobuf_src::protoc());
    #[cfg(feature = "protoc-bin-vendored")]
    std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path());
    prost_build::compile_protos(&["proto/glyphs.proto"], &["proto/"])?;
    Ok(())
}
