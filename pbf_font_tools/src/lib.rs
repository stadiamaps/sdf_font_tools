//! # PBF Font Tools
//!
//! Tools for working with SDF font glyphs in PBF format.
//!
//! This crate lets you combine multiple glyphs from multiple fonts into a single stack.
//! Generating glyphs from a TrueType/OpenType font (a la [node-fontnik](https://github.com/mapbox/node-fontnik))
//! is planned for a future release.
//!
//! ## References
//!   * [glyph-pbf-composite](https://github.com/mapbox/glyph-pbf-composite)
//!   * [tileserver-gl](https://github.com/klokantech/tileserver-gl/blob/master/src/utils.js)

mod error;
mod proto;
mod tools;

#[cfg(feature = "freetype")]
mod ft_generate;
#[cfg(feature = "freetype")]
pub use crate::ft_generate::*;

// Re-export freetype lib
#[cfg(feature = "freetype")]
pub use sdf_glyph_renderer::freetype;

pub use crate::error::PbfFontError;
pub use crate::tools::*;
pub use proto::glyphs::{Fontstack, Glyph, Glyphs};
