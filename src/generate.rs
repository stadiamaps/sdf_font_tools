use freetype::{Face, Library};

use crate::glyphs;

use sdf_glyph_renderer::{clamp_to_u8, render_sdf_from_face};

fn render_sdf_glyph(
    face: &Face,
    char_code: u32,
    buffer: usize,
    radius: usize,
    cutoff: f64,
) -> Result<glyphs::Glyph, sdf_glyph_renderer::Error> {
    let glyph = render_sdf_from_face(face, char_code, buffer, radius)?;

    let mut result = glyphs::Glyph::new();
    result.set_id(char_code);
    result.set_bitmap(clamp_to_u8(&glyph.sdf, cutoff));
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
    face: &Face,
    start: u32,
    end: u32,
    size: usize,
    radius: usize,
    cutoff: f64,
) -> Result<glyphs::Fontstack, sdf_glyph_renderer::Error> {
    if let Some(family_name) = face.family_name() {
        let mut stack = glyphs::Fontstack::new();

        let stack_name = if let Some(style_name) = face.style_name() {
            format!("{} {}", family_name, style_name)
        } else {
            family_name
        };

        stack.set_name(stack_name);
        stack.set_range(format!("{}-{}", start, end));

        // FreeType conventions: char width or height of zero means "use the same value"
        // and setting both resolution values to zero results in the default value
        // of 72 dpi.
        //
        // See https://www.freetype.org/freetype2/docs/reference/ft2-base_interface.html#ft_set_char_size
        // and https://www.freetype.org/freetype2/docs/tutorial/step1.html for details.
        face.set_char_size(0, (size << 6) as isize, 0, 0).unwrap();

        for char_code in start..=end {
            let result = render_sdf_glyph(face, char_code, 3, radius, cutoff);
            match result {
                Ok(glyph) => {
                    stack.glyphs.push(glyph);
                }
                Err(sdf_glyph_renderer::Error::FreeTypeError(
                    freetype::Error::InvalidGlyphIndex,
                )) => {
                    // Do nothing; not all glyphs will be present in a font.
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        Ok(stack)
    } else {
        // TODO: A real error haha
        Err(freetype::Error::MissingFontField.into())
    }
}

pub fn glyph_range_for_font(
    font_path: &std::path::Path,
    start: u32,
    end: u32,
    size: usize,
    radius: usize,
    cutoff: f64,
) -> Result<glyphs::Glyphs, sdf_glyph_renderer::Error> {
    let lib = Library::init().unwrap();
    let mut face = lib.new_face(font_path, 0).unwrap();
    let num_faces = face.num_faces();

    let mut result = glyphs::Glyphs::new();

    for face_index in 0..num_faces {
        if face_index > 0 {
            face = lib.new_face(font_path, face_index as isize).unwrap();
        }

        let stack = glyph_range_for_face(&face, start, end, size, radius, cutoff)?;
        result.stacks.push(stack);
    }

    Ok(result)
}
