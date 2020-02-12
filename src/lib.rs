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

#![deny(warnings)]

use std::collections::HashSet;
use std::path::Path;

use futures::future::try_join_all;

use tokio::fs::File;
use tokio::io::AsyncReadExt;

mod glyphs;

type GlyphResult = Result<glyphs::glyphs, protobuf::ProtobufError>;

/// Generates a single combined font stack for the set of fonts provided.
///
/// See the documentation for `combine_glyphs` for further details.
/// Unlike `combine_glyphs`, the result of this method will always contain a `glyphs` message,
/// even if the loaded range is empty for a given font.
pub async fn get_font_stack(
    font_path: &Path,
    font_names: Vec<String>,
    start: u32,
    end: u32,
) -> GlyphResult {
    // Load fonts (futures)
    let mut load_futures: Vec<_> = vec![];
    for font in font_names.iter() {
        load_futures.push(load_glyphs(font_path, font, start, end));
    }

    // Combine all the glyphs into a single instance, using the ordering to determine priority.
    match try_join_all(load_futures).await {
        Ok(data) => {
            if let Some(result) = combine_glyphs(data) {
                Ok(result)
            } else {
                // Construct an empty message manually if the range is not covered
                let mut result = glyphs::glyphs::new();

                let mut stack = glyphs::fontstack::new();
                stack.set_name(font_names.join(", "));
                stack.set_range(format!("{}-{}", start, end));

                result.mut_stacks().push(stack);
                Ok(result)
            }
        }
        Err(e) => Err(e),
    }
}

/// Loads a single font PBF slice from disk.
///
/// Fonts are assumed to be stored in <font_path>/<font_name>/<start>-<end>.pbf.
pub async fn load_glyphs(font_path: &Path, font_name: &str, start: u32, end: u32) -> GlyphResult {
    let full_path = font_path
        .join(font_name)
        .join(format!("{}-{}.pbf", start, end));
    let mut file = File::open(full_path).await?;
    let mut buf = Vec::new();

    file.read_to_end(&mut buf).await?;
    protobuf::parse_from_bytes(&buf)
}

/// Combines a list of SDF font glyphs into a single glyphs message.
/// All input font stacks are flattened into a single font stack containing all the glyphs.
/// The input order indicates precedence. If the same glyph ID is encountered multiple times,
/// only the first will be used.
///
/// NOTE: This returns `None` if there are no glyphs in the range. If you need to
/// construct an empty message, the responsibility lies with the caller.
pub fn combine_glyphs(data: Vec<glyphs::glyphs>) -> Option<glyphs::glyphs> {
    let mut result = glyphs::glyphs::new();
    let mut combined_stack = glyphs::fontstack::new();
    let mut coverage: HashSet<u32> = HashSet::new();

    for glyph_stack in data {
        for font_stack in glyph_stack.get_stacks() {
            if !combined_stack.has_name() {
                combined_stack.set_name(String::from(font_stack.get_name()));
            } else {
                let stack_name = combined_stack.get_name().to_owned();
                let this_name = font_stack.get_name();
                combined_stack.set_name(format!("{}, {}", stack_name, this_name));
            }

            for glyph in font_stack.get_glyphs() {
                if !coverage.contains(&glyph.get_id()) {
                    coverage.insert(glyph.get_id());
                    combined_stack.mut_glyphs().push(glyph.to_owned());
                }
            }
        }
    }

    let start = coverage.iter().min()?;
    let end = coverage.iter().min()?;

    combined_stack.set_range(format!("{}-{}", start, end));

    result.mut_stacks().push(combined_stack);

    Some(result)
}
