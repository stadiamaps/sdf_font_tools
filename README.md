# Rust PBF Font Tools

[![](https://img.shields.io/crates/v/pbf_font_tools.svg)](https://crates.io/crates/pbf_font_tools) [![](https://docs.rs/pbf_font_tools/badge.svg)](https://docs.rs/pbf_font_tools)

This crate contains tools for working with SDF font glyphs in PBF format for use in renderers
like Mapbox GL.

## Features

* Combine multiple glyphs from multiple fonts into a single stack. 
* Generate glyphs from a TrueType/OpenType font.

If you're looking for a CLI tool to generate PBF ranges en mass like
[node-fontnik](https://github.com/mapbox/node-fontnik)), but faster,
check out []

## References

* https://github.com/mapbox/glyph-pbf-composite
* https://github.com/klokantech/tileserver-gl/blob/master/src/utils.js
