//! # PBF Font Tools
//!
//! Tools for working with SDF font glyphs in PBF format.
//!
//! This crate lets you combine multiple glyphs from multiple fonts into a single stack.
//! Generating glyphs from a TrueType/OpenType font (a la [node-fontnik](https://github.com/mapbox/node-fontnik))
//! is planned for a future release.
//!
//! ## References
//!   * https://github.com/mapbox/glyph-pbf-composite
//!   * https://github.com/klokantech/tileserver-gl/blob/master/src/utils.js

pub use proto::glyphs;

use futures::future::try_join_all;
use protobuf::Message;
use std::{collections::HashSet, fs::File, path::Path};
use tokio::task::spawn_blocking;

#[cfg(feature = "freetype")]
pub mod generate;

mod proto;

type GlyphResult = Result<glyphs::glyphs, protobuf::ProtobufError>;

/// Generates a single combined font stack for the set of fonts provided.
///
/// See the documentation for `combine_glyphs` for further details.
/// Unlike `combine_glyphs`, the result of this method will always contain a `glyphs` message,
/// even if the loaded range is empty for a given font.
pub async fn get_font_stack(
    font_path: &Path,
    font_names: &[&str],
    start: u32,
    end: u32,
) -> GlyphResult {
    // Load fonts
    let glyph_data = try_join_all(
        font_names
            .iter()
            .map(|font| load_glyphs(font_path, font, start, end)),
    )
    .await?;

    // Combine all the glyphs into a single instance, using the ordering to determine priority.
    // This can take some time, so mark it blocking.
    Ok(spawn_blocking(move || combine_glyphs(glyph_data))
        .await
        .unwrap() // Unwrap any panics.
        .unwrap_or_else(|| {
            // Construct an empty message manually if the range is not covered
            let mut result = glyphs::glyphs::new();

            let mut stack = glyphs::fontstack::new();
            stack.set_name(font_names.join(", "));
            stack.set_range(format!("{}-{}", start, end));

            result.mut_stacks().push(stack);
            result
        }))
}

/// Loads a single font PBF slice from disk.
///
/// Fonts are assumed to be stored in <font_path>/<font_name>/<start>-<end>.pbf.
pub async fn load_glyphs(font_path: &Path, font_name: &str, start: u32, end: u32) -> GlyphResult {
    let full_path = font_path
        .join(font_name)
        .join(format!("{}-{}.pbf", start, end));

    // Note: Counter-intuitively, it's much faster to use blocking IO with `spawn_blocking` here,
    // since the `Message::parse_` call will block as well.
    spawn_blocking(|| {
        let mut file = File::open(full_path)?;
        Message::parse_from_reader(&mut file)
    })
    .await
    .unwrap() // Unwrap any panics.
}

/// Combines a list of SDF font glyphs into a single glyphs message.
/// All input font stacks are flattened into a single font stack containing all the glyphs.
/// The input order indicates precedence. If the same glyph ID is encountered multiple times,
/// only the first will be used.
///
/// NOTE: This returns `None` if there are no glyphs in the range. If you need to
/// construct an empty message, the responsibility lies with the caller.
pub fn combine_glyphs(glyphs_to_combine: Vec<glyphs::glyphs>) -> Option<glyphs::glyphs> {
    let mut result = glyphs::glyphs::new();
    let mut combined_stack = glyphs::fontstack::new();
    let mut coverage: HashSet<u32> = HashSet::new();

    for mut glyph_stack in glyphs_to_combine {
        for mut font_stack in glyph_stack.take_stacks() {
            if !combined_stack.has_name() {
                combined_stack.set_name(font_stack.take_name());
            } else {
                let name = combined_stack.mut_name();
                name.push_str(", ");
                name.push_str(font_stack.get_name());
            }

            for glyph in font_stack.take_glyphs() {
                if !coverage.contains(&glyph.get_id()) {
                    coverage.insert(glyph.get_id());
                    combined_stack.mut_glyphs().push(glyph);
                }
            }
        }
    }

    let start = coverage.iter().min()?;
    let end = coverage.iter().max()?;

    combined_stack.set_range(format!("{}-{}", start, end));

    result.mut_stacks().push(combined_stack);

    Some(result)
}
