//! # PBF Font Tools
//!
//! Tools for working with SDF font glyphs in PBF format.
//!
//! This crate lets you combine multiple glyphs from multiple fonts into a single stack.
//! The optional `generate` feature enables glyph generation from TrueType/OpenType fonts
//! (a la [node-fontnik](https://github.com/mapbox/node-fontnik)).
//!
//! ## References
//!   * [glyph-pbf-composite](https://github.com/mapbox/glyph-pbf-composite)
//!   * [tileserver-gl](https://github.com/klokantech/tileserver-gl/blob/master/src/utils.js)

mod error;
mod proto;
mod tools;

#[cfg(feature = "generate")]
mod font_generate;

// Re-export protobuf lib
pub use prost;
pub use proto::{Fontstack, Glyph, Glyphs};
#[cfg(feature = "generate")]
pub use sdf_glyph_renderer::FontFace;

pub use crate::error::PbfFontError;
#[cfg(feature = "generate")]
pub use crate::font_generate::*;
pub use crate::tools::*;
