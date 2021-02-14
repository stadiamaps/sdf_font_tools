# Protobuf SDF Font Glyph Builder

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

## Usage

This tool will create `out_dir` if necessary, and will put each range (of 256 glyphs, for
compatibility with Mapbox fontstack convention) in a new subdirectory bearing the font name.
**Any existing glyphs will be overwritten in place.**

```
$ cargo run --release -- /path/to/font_dir /path/to/out_dir
```
