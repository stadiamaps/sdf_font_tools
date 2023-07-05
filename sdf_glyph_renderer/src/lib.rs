//! This crate is a Rust implementation of the signed distance field generation techniques
//! demonstrated by [Valve](https://steamcdn-a.akamaihd.net/apps/valve/2007/SIGGRAPH2007_AlphaTestedMagnification.pdf)
//! and [Mapbox](https://blog.mapbox.com/drawing-text-with-signed-distance-fields-in-mapbox-gl-b0933af6f817).
//! The generic interface works with any bitmap, and a high level interface enables easy operation
//! with FreeType faces when the optional `freetype` feature is enabled.
//!
//! The approach taken by this crate is similar to [TinySDF](https://github.com/mapbox/tiny-sdf);
//! it works from a raster bitmap rather than directly from vector outlines. This keeps the
//! crate simple and allows it to be used generically with any bitmap. The SDF is calculated
//! using the same algorithm described in [this paper](http://cs.brown.edu/people/pfelzens/papers/dt-final.pdf)
//! by Felzenszwalb & Huttenlocher.
//!
//! Rather than re-invent the rasterisation process for fonts, this crate relies on FreeType to
//! generate the bitmap. This is quite fast (we're talking µs/glyph), and the results are
//! almost always indistinguishable from the more sophisticated vector-based approach of
//! [sdf-glyph-foundry](https://github.com/mapbox/sdf-glyph-foundry).
//!
//! This crate is used by [pbf_font_tools](https://github.com/stadiamaps/pbf_font_tools) to generate
//! SDF glyphs from any FreeType-readable font. If you're looking for a batch generation tool,
//! check out [build_pbf_glyphs](https://github.com/stadiamaps/build_pbf_glyphs).

mod core;
pub use crate::core::*;

mod error;
pub use crate::error::SdfGlyphError;

#[cfg(feature = "freetype")]
mod ft;
#[cfg(feature = "freetype")]
pub use crate::ft::*;

// Re-export freetype crate if the feature is enabled
#[cfg(feature = "freetype")]
pub use freetype;
