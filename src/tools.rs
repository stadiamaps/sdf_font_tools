use crate::proto::glyphs::{Fontstack, Glyphs};
use crate::PbfFontError;
use futures::future::try_join_all;
use protobuf::Message;
use std::collections::HashSet;
use std::fs::File;
use std::path::Path;
use tokio::task::spawn_blocking;

/// Generates a single combined font stack for the set of fonts provided.
///
/// See the documentation for `combine_glyphs` for further details.
/// Unlike `combine_glyphs`, the result of this method will always contain a `glyphs` message,
/// even if the loaded range is empty for a given font.
pub async fn get_named_font_stack<P: AsRef<Path>>(
    font_path: P,
    font_names: &[&str],
    stack_name: String,
    start: u32,
    end: u32,
) -> Result<Glyphs, PbfFontError> {
    // Load fonts
    let glyph_data = try_join_all(
        font_names
            .into_iter()
            .map(|font| load_glyphs(font_path.as_ref(), font, start, end)),
    )
    .await?;

    // Combine all the glyphs into a single instance, using the ordering to determine priority.
    // This can take some time, so mark it blocking.
    Ok(spawn_blocking(move || combine_glyphs(glyph_data))
        .await?
        .unwrap_or_else(|| {
            // Construct an empty message manually if the range is not covered
            let mut result = Glyphs::new();

            let mut stack = Fontstack::new();
            stack.set_name(stack_name);
            stack.set_range(format!("{start}-{end}"));

            result.stacks.push(stack);
            result
        }))
}

pub async fn get_font_stack<P: AsRef<Path>>(
    font_path: P,
    font_names: &[&str],
    start: u32,
    end: u32,
) -> Result<Glyphs, PbfFontError> {
    let stack_name = font_names.join(", ");
    get_named_font_stack(font_path, font_names, stack_name, start, end).await
}

/// Loads a single font PBF slice from disk.
///
/// Fonts are assumed to be stored in `<font_path>/<font_name>/<start>-<end>.pbf`.
pub async fn load_glyphs<P: AsRef<Path>>(
    font_path: P,
    font_name: &str,
    start: u32,
    end: u32,
) -> Result<Glyphs, PbfFontError> {
    let full_path = font_path
        .as_ref()
        .join(font_name)
        .join(format!("{start}-{end}.pbf"));

    // Note: Counter-intuitively, it's much faster to use blocking IO with `spawn_blocking` here,
    // since the `Message::parse_` call will block as well.
    Ok(spawn_blocking(|| {
        let mut file = File::open(full_path)?;
        Message::parse_from_reader(&mut file)
    })
    .await??)
}

/// Combines a list of SDF font glyphs into a single glyphs message.
/// All input font stacks are flattened into a single font stack containing all the glyphs.
/// The input order indicates precedence. If the same glyph ID is encountered multiple times,
/// only the first will be used.
///
/// NOTE: This returns `None` if there are no glyphs in the range. If you need to
/// construct an empty message, the responsibility lies with the caller.
#[must_use]
pub fn combine_glyphs(glyphs_to_combine: Vec<Glyphs>) -> Option<Glyphs> {
    let mut result = Glyphs::new();
    let mut combined_stack = Fontstack::new();
    let mut coverage: HashSet<u32> = HashSet::new();

    for mut glyph_stack in glyphs_to_combine {
        for mut font_stack in glyph_stack.stacks.drain(..) {
            if combined_stack.has_name() {
                let name = combined_stack.mut_name();
                name.push_str(", ");
                name.push_str(&font_stack.take_name());
            } else {
                combined_stack.set_name(font_stack.take_name());
            }

            for glyph in font_stack.glyphs.drain(..) {
                if let Some(id) = glyph.id {
                    if !coverage.contains(&id) {
                        coverage.insert(id);
                        combined_stack.glyphs.push(glyph);
                    }
                }
            }
        }
    }

    let start = coverage.iter().min()?;
    let end = coverage.iter().max()?;

    combined_stack.set_range(format!("{start}-{end}"));

    result.stacks.push(combined_stack);

    Some(result)
}
