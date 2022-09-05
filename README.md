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

You can install the released version from crates.io, or grab the latest git version by
running `cargo install --git https://github.com/stadiamaps/build_pbf_glyphs build_pbf_glyphs`. 

```
$ build_pbf_glyphs /path/to/font_dir /path/to/out_dir
```

### Overwriting existing glyphs

By default, existing glyphs will **not** be overwritten as this is normally a waste of CPU.
You can change this by adding the `--overwrite` flag.

### Combining glyphs upfront

For some applications, it may be desirable to combine glyphs upfront. While this is a cheap
operation, computationally speaking, this may be convenient if you want to keep your server logic
simple. If you only use one font in the list, simple static file serving of a directory will
suffice.

This tool can pre-combine the glyphs for you using the `-c <spec.json>` command line switch.
The file should contain a JSON dictionary having a format like so:

```json
{
  "New Font Name": ["Font 1", "Font 2"]
}
```

This is run as a separate pass after all glyphs have been generated, so all fonts are assumed to
have valid glyphs already in `out_dir`.