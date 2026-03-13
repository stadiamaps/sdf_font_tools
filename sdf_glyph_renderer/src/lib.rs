//! This crate is a Rust implementation of the signed distance field generation techniques
//! demonstrated by [Valve](https://steamcdn-a.akamaihd.net/apps/valve/2007/SIGGRAPH2007_AlphaTestedMagnification.pdf)
//! and [Mapbox](https://blog.mapbox.com/drawing-text-with-signed-distance-fields-in-mapbox-gl-b0933af6f817).
//! The generic interface works with any bitmap, and a high level interface enables easy operation
//! with FreeType faces when the optional `freetype` feature is enabled (bundles FreeType from source).
//! Use the `freetype-system` feature instead to link against a system-installed FreeType.
//! An alternative vector-based approach using bezier curves is available via the `ttf-parser` feature.
//!
//! The bitmap-based approach taken by this crate is similar to [TinySDF](https://github.com/mapbox/tiny-sdf);
//! it works from a raster bitmap rather than directly from vector outlines. The SDF is calculated
//! using the same algorithm described in [this paper](http://cs.brown.edu/people/pfelzens/papers/dt-final.pdf)
//! by Felzenszwalb & Huttenlocher.
//!
//! When the `ttf-parser` feature is enabled, an alternative vector-based approach computes the
//! SDF directly from the font's bezier curve outlines, similar to
//! [sdf-glyph-foundry](https://github.com/mapbox/sdf-glyph-foundry). This produces higher
//! quality results for complex scripts (Indic, Khmer, Burmese, etc.).
//!
//! This crate is used by [pbf_font_tools](https://github.com/stadiamaps/pbf_font_tools) to generate
//! SDF glyphs from any FreeType-readable font. If you're looking for a batch generation tool,
//! check out [build_pbf_glyphs](https://github.com/stadiamaps/build_pbf_glyphs).

mod core;
pub use crate::core::*;

mod error;
pub use crate::error::SdfGlyphError;

mod types;
pub use crate::types::*;

#[cfg(feature = "freetype")]
mod ft;

// Re-export freetype crate if the feature is enabled
#[cfg(feature = "freetype")]
pub use freetype;

#[cfg(feature = "freetype")]
pub use crate::ft::*;

// Vector-based SDF via ttf-parser
#[cfg(feature = "ttf-parser")]
pub mod outline;

#[cfg(feature = "ttf-parser")]
mod ttf;

#[cfg(feature = "ttf-parser")]
pub use ttf_parser;

#[cfg(feature = "ttf-parser")]
pub use crate::ttf::*;
