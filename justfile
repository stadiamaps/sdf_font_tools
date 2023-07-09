#!/usr/bin/env just --justfile

# Run cargo fmt and cargo clippy
lint: fmt clippy

# Run Nightly cargo fmt, ordering imports
fmt:
    cargo +nightly fmt -- --config imports_granularity=Module,group_imports=StdExternalCrate

# Run cargo clippy
clippy:
    cargo clippy --workspace --all-targets --bins --tests --lib --benches -- -D warnings
