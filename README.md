# Protobuf SDF Font Glyph Builder

[![](https://img.shields.io/crates/v/build_pbf_glyphs.svg)](https://crates.io/crates/build_pbf_glyphs)

This binary crate provides a CLI utility for batch converting a directory of fonts into
signed distance fields, encoded in a protocol buffer for renderers such as Mapbox GL. This
isn't really anything novel; it's just a frontend to
[pbf_font_tools](https://github.com/stadiamaps/pbf_font_tools) that behaves similar to
[node-fontnik](https://github.com/mapbox/node-fontnik), but is faster and (in our opinion)
a bit easier to use since it doesn't depend on node and all its headaches, or C++ libraries
that need to be built from scratch (this depends on FreeType, but that's widely available on
nearly any *nix-based system).

Check out
[sdf_glyph_renderer](https://github.com/stadiamaps/sdf_glyph_renderer) for more technical
details on how this works.

NOTE: This has requires you to have FreeType installed on your system. We recommend using
FreeType 2.10 or newer. Everything will still work against many older 2.x versions, but
the glyph generation improves over time so things will generally look better with newer
versions.

## Usage

This tool will create `out_dir` if necessary, and will put each range (of 256 glyphs, for
compatibility with Mapbox fontstack convention) in a new subdirectory bearing the font name.
**Any existing glyphs will be overwritten in place.**

You can install the released version from crates.io, or grab the latest git version by
running `cargo install --git https://github.com/stadiamaps/build_pbf_glyphs build_pbf_glyphs`. 

```
$ build_pbf_glyphs /path/to/font_dir /path/to/out_dir
```
