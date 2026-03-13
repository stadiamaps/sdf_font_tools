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
#[cfg(feature = "ttf-parser")]
mod ttf_generate;

pub use proto::{Fontstack, Glyph, Glyphs};
// Re-export protobuf lib
pub use prost;
// Re-export freetype lib
#[cfg(feature = "freetype")]
pub use sdf_glyph_renderer::freetype;
// Re-export ttf-parser lib
#[cfg(feature = "ttf-parser")]
pub use sdf_glyph_renderer::ttf_parser;

pub use crate::error::PbfFontError;
#[cfg(feature = "freetype")]
pub use crate::ft_generate::*;
#[cfg(feature = "ttf-parser")]
pub use crate::ttf_generate::*;
pub use crate::tools::*;
