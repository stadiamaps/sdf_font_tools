# SDF Glyph Renderer

[![](https://img.shields.io/crates/v/sdf_glyph_renderer.svg)](https://crates.io/crates/sdf_glyph_renderer) [![](https://docs.rs/sdf_glyph_renderer/badge.svg)](https://docs.rs/sdf_glyph_renderer)

This crate is a Rust implementation of the signed distance field generation techniques
demonstrated by [Valve](https://steamcdn-a.akamaihd.net/apps/valve/2007/SIGGRAPH2007_AlphaTestedMagnification.pdf)
and [Mapbox](https://blog.mapbox.com/drawing-text-with-signed-distance-fields-in-mapbox-gl-b0933af6f817).
The generic interface works with any bitmap, and a high level interface enables easy operation
with FreeType faces when the optional `freetype` feature is enabled.

The approach taken by this crate is similar to [TinySDF](https://github.com/mapbox/tiny-sdf);
it works from a raster bitmap rather than directly from vector outlines. This keeps the
crate simple and allows it to be used generically with any bitmap. The SDF is calculated
using the same algorithm described in [this paper](http://cs.brown.edu/people/pfelzens/papers/dt-final.pdf)
by Felzenszwalb & Huttenlocher.

Rather than re-invent the rasterisation process for fonts, this crate relies on FreeType to
generate the bitmap. This is quite fast (we're talking µs/glyph), and the results are
almost always indistinguishable from the more sophisticated vector-based approach of
[sdf-glyph-foundry](https://github.com/mapbox/sdf-glyph-foundry).

This crate is used by [pbf_font_tools](https://github.com/stadiamaps/sdf_font_tools/tree/main/pbf_font_tools) to generate
SDF glyphs from any FreeType-readable font. If you're looking for a batch generation tool,
check out [build_pbf_glyphs](https://github.com/stadiamaps/sdf_font_tools/tree/main/build_pbf_glyphs).

# Example Usage

```rust
// The alpha raster is a Vec<u8> expressing the level of opacity at each pixel.
// In this case, the following map is for the # symbol from Open Sans Light.
// This bitmap is pre-buffered with a 3px border around it, which we note below.
let alpha = Vec::from([
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 27, 75, 91, 55, 2, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 3, 141, 246, 196, 180, 231, 205, 38, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 117, 236, 42, 0, 0, 10, 180, 207, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    203, 130, 0, 0, 0, 0, 46, 255, 29, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 215, 107, 0,
    0, 0, 0, 33, 255, 31, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 163, 169, 0, 0, 0, 0, 113,
    221, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 43, 247, 71, 0, 0, 65, 240, 71, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 96, 241, 69, 138, 231, 71, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 8, 199, 255, 191, 22, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 60, 222, 186, 136, 242, 58, 0, 0, 0, 0, 0, 8, 32, 0, 0, 0, 0, 0, 0, 0, 0, 85,
    240, 92, 0, 0, 117, 241, 56, 0, 0, 0, 0, 105, 175, 0, 0, 0, 0, 0, 0, 0, 17, 240, 79, 0,
    0, 0, 0, 118, 240, 54, 0, 0, 0, 199, 81, 0, 0, 0, 0, 0, 0, 0, 100, 210, 0, 0, 0, 0, 0,
    0, 118, 239, 52, 0, 73, 230, 6, 0, 0, 0, 0, 0, 0, 0, 134, 173, 0, 0, 0, 0, 0, 0, 0,
    119, 238, 64, 226, 84, 0, 0, 0, 0, 0, 0, 0, 0, 123, 196, 0, 0, 0, 0, 0, 0, 0, 0, 120,
    255, 182, 0, 0, 0, 0, 0, 0, 0, 0, 0, 54, 252, 45, 0, 0, 0, 0, 0, 0, 7, 163, 230, 235,
    46, 0, 0, 0, 0, 0, 0, 0, 0, 0, 159, 230, 92, 4, 0, 0, 14, 90, 218, 189, 17, 120, 235,
    44, 0, 0, 0, 0, 0, 0, 0, 0, 3, 118, 231, 251, 222, 227, 251, 197, 91, 2, 0, 0, 125,
    233, 43, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 34, 55, 49, 17, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
]);

// Construct a new bitmap using the alpha bitmap, and the raw metrics: a width of 16px,
// a height of 19px, and a 3px buffer.
let bitmap = BitmapGlyph::new(alpha, 16, 19, 3);

// Generate the signed distance field from the bitmap
let sdf = render_sdf(&bitmap, 8);
```
