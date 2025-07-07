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

By default, this crate uses a vendored protobuf compiler binary (`protoc`)
to support the widest number of platforms.
You can disable the default features to opt out of this.

When opted out of the vendored compiler, you can ensure `protoc` is accessible in any of the following ways:

* Disabling the default features will look for `protoc` in your `PATH` by default.
* To build from source (requires a C++ compiler), enable the `protoc-from-src` feature. This will be used instead.
* To use a specific `protoc` that isn't in your `PATH`, set the `PROTOC` environment variable during build.
