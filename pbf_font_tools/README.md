# Rust PBF Font Tools

[![](https://img.shields.io/crates/v/pbf_font_tools.svg)](https://crates.io/crates/pbf_font_tools) [![](https://docs.rs/pbf_font_tools/badge.svg)](https://docs.rs/pbf_font_tools)

This crate contains tools for working with SDF font glyphs in PBF format for use in renderers
like Mapbox GL.

## Features

* Combine multiple glyphs from multiple fonts into a single stack. 
* Generate glyphs from a TrueType/OpenType font.

If you're looking for a CLI tool to generate PBF ranges en masse like
[node-fontnik](https://github.com/mapbox/node-fontnik)), but faster,
check out [build_pbf_glyphs](https://github.com/stadiamaps/sdf_font_tools/tree/main/build_pbf_glyphs).

NOTE: This has been developed and tested against FreeType 2.10. It will work against
older versions, but the glyph generation tests may not pass as the rendering
evolves over time.

## References

* https://github.com/mapbox/glyph-pbf-composite
* https://github.com/klokantech/tileserver-gl/blob/master/src/utils.js

## protoc

By default, this crate will build the protobuf compiler from source during build on most platforms
to encourage better reproducibility.
This requires having a C++ compiler at the moment.

You can change this behavior with feature flags in the following ways:

* Disabling default flags will disable the source build (be sure to add freetype if you need it!).
* To use a specific `protoc` that you have installed on your system, set the `PROTOC` environment variable during build.
  (Otherwise, your `PATH` will be checked to find one).
* To skip builds and use a vendored `protoc` binary, enable the `protoc-bin-vendored` feature.
