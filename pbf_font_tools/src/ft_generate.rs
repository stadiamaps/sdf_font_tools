use std::path::Path;

use sdf_glyph_renderer::{clamp_to_u8, render_sdf_from_face};

use crate::error::PbfFontError;
use crate::{freetype, Fontstack, Glyph, Glyphs};

/// Renders a single glyph for the given font face into a Glyph message.
pub fn render_sdf_glyph(
    face: &freetype::Face,
    char_code: u32,
    buffer: usize,
    radius: usize,
    cutoff: f64,
) -> Result<Glyph, PbfFontError> {
    let glyph = render_sdf_from_face(face, char_code, buffer, radius)?;

    let mut result = Glyph::new();
    result.set_id(char_code);
    result.set_bitmap(clamp_to_u8(&glyph.sdf, cutoff)?);
    result.set_width(glyph.metrics.width as u32);
    result.set_height(glyph.metrics.height as u32);
    result.set_left(glyph.metrics.left_bearing);
    result.set_top(glyph.metrics.top_bearing - glyph.metrics.ascender);
    result.set_advance(glyph.metrics.h_advance);

    Ok(result)
}

/// Renders a glyph range for the given font face into a Mapbox-compatible fontstack.
///
/// The `radius` and `cutoff` parameters are exposed in case you are working with an
/// alternate renderer with tunable options, but you are probably best off sticking
/// with 8 and 0.25 respectively.
///
/// The `radius` controls how many pixels out from the font outline to record distances
/// from the font outline (the rest will be clamped to zero). `cutoff` controls what
/// percentage of values will be used to record the negative values (since the SDF is
/// encoded as a vector of bytes, which have no sign). The value selected must be
/// between 0 and 1.
pub fn glyph_range_for_face(
    face: &freetype::Face,
    start: u32,
    end: u32,
    size: usize,
    radius: usize,
    cutoff: f64,
) -> Result<Fontstack, PbfFontError> {
    let Some(mut family_name) = face.family_name() else {
        return Err(PbfFontError::MissingFontFamilyName)?;
    };
    if let Some(style_name) = face.style_name() {
        family_name.push(' ');
        family_name.push_str(&style_name);
    }

    let mut stack = Fontstack::new();
    stack.set_name(family_name);
    stack.set_range(format!("{start}-{end}"));

    // FreeType conventions: char width or height of zero means "use the same value"
    // and setting both resolution values to zero results in the default value
    // of 72 dpi.
    //
    // See https://www.freetype.org/freetype2/docs/reference/ft2-base_interface.html#ft_set_char_size
    // and https://www.freetype.org/freetype2/docs/tutorial/step1.html for details.
    face.set_char_size(0, (size << 6) as isize, 0, 0)?;

    for char_code in start..=end {
        match render_sdf_glyph(face, char_code, 3, radius, cutoff) {
            Ok(glyph) => {
                stack.glyphs.push(glyph);
            }
            Err(PbfFontError::SdfGlyphError(sdf_glyph_renderer::SdfGlyphError::FreeTypeError(
                freetype::Error::InvalidGlyphIndex,
            ))) => {
                // Do nothing; not all glyphs will be present in a font.
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    Ok(stack)
}

pub fn glyph_range_for_font<P: AsRef<Path>>(
    font_path: P,
    start: u32,
    end: u32,
    size: usize,
    radius: usize,
    cutoff: f64,
) -> Result<Glyphs, PbfFontError> {
    let lib = freetype::Library::init()?;
    let mut face = lib.new_face(font_path.as_ref(), 0)?;
    let num_faces = face.num_faces();

    let mut result = Glyphs::new();
    result.stacks.reserve(num_faces as usize);

    for face_index in 0..num_faces {
        if face_index > 0 {
            face = lib.new_face(font_path.as_ref(), face_index as isize)?;
        }

        let stack = glyph_range_for_face(&face, start, end, size, radius, cutoff)?;
        result.stacks.push(stack);
    }

    Ok(result)
}
