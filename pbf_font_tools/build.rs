use std::io::Result;

fn main() -> Result<()> {
    // protoc compiled from source
    #[cfg(feature = "protoc-from-src")]
    std::env::set_var("PROTOC", protobuf_src::protoc());

    // Vendored protoc binary
    #[cfg(feature = "protoc-vendored")]
    std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path().expect("Unable to find protoc!"));

    prost_build::compile_protos(&["proto/glyphs.proto"], &["proto/"])?;
    Ok(())
}
