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
extern crate futures;
extern crate protobuf;
extern crate tokio;

use futures::Future;

use std::collections::HashSet;
use std::path::Path;

use tokio::fs::File;
use tokio::io;

mod glyphs;

type GlyphFuture = Box<Future<Item = glyphs::glyphs, Error = protobuf::ProtobufError> + Send>;

/// Generates a single combined font stack for the set of fonts provided.
///
/// See the documentation for `combine_glyphs` for further details.
/// Unlike `combine_glyphs`, the result of this method will always contain a `glyphs` message,
/// even if the loaded range is empty for a given font.
pub fn get_font_stack(
    font_path: &Path,
    font_names: Vec<String>,
    start: u32,
    end: u32,
) -> GlyphFuture {
    // Load fonts (futures)
    let load_futures: Vec<GlyphFuture> = font_names
        .iter()
        .map(|font| -> GlyphFuture { load_glyphs(font_path, font, start, end) })
        .collect();

    // Combine all the glyphs into a single instance, using the ordering to determine priority.
    let combinator = futures::future::join_all(load_futures);
    Box::new(combinator.map(move |data| -> glyphs::glyphs {
        if let Some(result) = combine_glyphs(data) {
            result
        } else {
            // Construct an empty message manually if the range is not covered
            let mut result = glyphs::glyphs::new();

            let mut stack = glyphs::fontstack::new();
            stack.set_name(font_names.join(", "));
            stack.set_range(format!("{}-{}", start, end));

            result.mut_stacks().push(stack);
            result
        }
    }))
}

/// Loads a single font PBF slice from disk.
///
/// Fonts are assumed to be stored in <font_path>/<font_name>/<start>-<end>.pbf.
pub fn load_glyphs(font_path: &Path, font_name: &str, start: u32, end: u32) -> GlyphFuture {
    let full_path = font_path
        .join(font_name)
        .join(format!("{}-{}.pbf", start, end));
    let fut = File::open(full_path)
        .or_else(|e| futures::future::err(protobuf::ProtobufError::from(e)))
        .and_then(|file| {
            io::read_to_end(file, Vec::new())
                .or_else(|e| {
                    futures::future::err(protobuf::ProtobufError::from(e))
                })
                .and_then(|(_file, data)| -> protobuf::ProtobufResult<glyphs::glyphs> {
                    protobuf::parse_from_bytes(&data)
                })
        });
    Box::new(fut)
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
                let local_stack = combined_stack.clone();
                let stack_name = local_stack.get_name();
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

    return Some(result);
}
