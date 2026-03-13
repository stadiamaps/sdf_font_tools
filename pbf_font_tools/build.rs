use std::io::Result;

fn main() -> Result<()> {
    eprintln!("Protobuf features: protoc-from-src: {}, protoc-vendored: {}", cfg!(feature = "protoc-from-src"), cfg!(feature = "protoc-vendored"));

    if cfg!(feature = "protoc-from-src") && cfg!(feature = "protoc-vendored") {
        panic!("It looks like you've enabled both protoc-from-src and protoc-vendored at the same time. You probably want to pick just one.\n\n(Hint: did you forget to add default-features = false in your Cargo.toml?)");
    }

    // protoc compiled from source
    #[cfg(feature = "protoc-from-src")]
    std::env::set_var("PROTOC", protobuf_src::protoc());

    // Vendored protoc binary
    #[cfg(feature = "protoc-vendored")]
    std::env::set_var(
        "PROTOC",
        protoc_bin_vendored::protoc_bin_path().expect("Unable to find protoc!"),
    );

    prost_build::compile_protos(&["proto/glyphs.proto"], &["proto/"])?;
    Ok(())
}
