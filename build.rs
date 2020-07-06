use protobuf_codegen_pure::Codegen;

use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn out_dir() -> PathBuf {
    Path::new(&env::var("OUT_DIR").expect("Missing OUT_DIR env")).join("proto")
}

fn cleanup() {
    let _ = fs::remove_dir_all(&out_dir());
}

fn compile() {
    let proto_dir =
        Path::new(&env::var("CARGO_MANIFEST_DIR").expect("Missing CARGO_MANIFEST_DIR env"))
            .join("proto");

    let files = glob::glob(&proto_dir.join("**/*.proto").to_string_lossy())
        .expect("glob")
        .filter_map(|p| p.ok().map(|p| p.to_string_lossy().into_owned()))
        .collect::<Vec<_>>();

    let out_dir = out_dir();
    fs::create_dir(&out_dir).expect("Unable to create directory");

    Codegen::new()
        .out_dir(out_dir)
        .inputs(files)
        .include(proto_dir)
        .run()
        .expect("Protobuf codegen failed.");
}

fn generate_mod_rs() {
    let out_dir = out_dir();

    let mods = glob::glob(&out_dir.join("*.rs").to_string_lossy())
        .expect("glob")
        .filter_map(|p| {
            p.ok()
                .map(|p| format!("pub mod {};", p.file_stem().unwrap().to_string_lossy()))
        })
        .collect::<Vec<_>>()
        .join("\n");

    let mod_rs = out_dir.join("mod.rs");
    fs::write(&mod_rs, format!("// @generated\n{}\n", mods)).expect("write");

    println!("cargo:rustc-env=PROTO_MOD_RS={}", mod_rs.to_string_lossy());
}

fn main() {
    cleanup();
    compile();
    generate_mod_rs();
}
