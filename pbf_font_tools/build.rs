fn main() {
    #[cfg(feature = "force-protobuf-gen")]
    generate_proto().expect("failed to regenerate protobuf source");
}

#[cfg(feature = "force-protobuf-gen")]
fn generate_proto() -> std::io::Result<()> {
    use std::path::Path;

    eprintln!(
        "Protobuf features: protoc-from-src: {}, protoc-vendored: {}",
        cfg!(feature = "protoc-from-src"),
        cfg!(feature = "protoc-vendored")
    );

    if cfg!(feature = "protoc-from-src") && cfg!(feature = "protoc-vendored") {
        panic!("It looks like you've enabled both protoc-from-src and protoc-vendored at the same time. You probably want to pick just one.\n\n(Hint: did you forget to add default-features = false in your Cargo.toml?)");
    }

    // protoc compiled from source
    #[cfg(feature = "protoc-from-src")]
    std::env::set_var("PROTOC", protobuf_src::protoc());

    // Vendored protoc binary
    #[cfg(feature = "protoc-vendored")]
    if let Ok(vendored_protoc_path) = protoc_bin_vendored::protoc_bin_path() {
        std::env::set_var("PROTOC", vendored_protoc_path);
    } else {
        eprintln!(
            "Failed to find vendored protoc binary; trying to fall back to protoc in $PATH..."
        );
    }

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let proto_dir = Path::new(&manifest_dir).join("proto");
    let proto_file = proto_dir.join("glyphs.proto");
    // Write the generated .rs file directly into the source tree so it can be vendored in git.
    // In CI, `git diff --exit-code` detects any drift between the .proto definition and the
    // committed generated file.
    let out_dir = Path::new(&manifest_dir).join("src/proto");

    prost_build::Config::new()
        .out_dir(&out_dir)
        .compile_protos(&[&proto_file], &[&proto_dir])?;

    Ok(())
}
