use std::path::Path;
use std::sync::Arc;

use sdf_glyph_renderer::{clamp_to_u8, FontFace, FontRenderer};

use crate::error::PbfFontError;
use crate::{Fontstack, Glyph, Glyphs};

/// Renders a single glyph for the given font face into a Glyph message.
pub fn render_sdf_glyph(
    renderer: &mut FontRenderer,
    face: &FontFace,
    char_code: u32,
    size: usize,
    buffer: usize,
    radius: usize,
    cutoff: f64,
) -> Result<Glyph, PbfFontError> {
    let glyph = renderer.render_sdf_from_face(face, char_code, size as f32, buffer, radius)?;

    Ok(Glyph {
        id: char_code,
        bitmap: Some(clamp_to_u8(&glyph.sdf, cutoff)?),
        width: glyph.metrics.width as u32,
        height: glyph.metrics.height as u32,
        left: glyph.metrics.left_bearing,
        top: glyph.metrics.top_bearing - glyph.metrics.ascender,
        advance: glyph.metrics.h_advance,
    })
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
    face: &FontFace,
    start: u32,
    end: u32,
    size: usize,
    radius: usize,
    cutoff: f64,
) -> Result<Fontstack, PbfFontError> {
    let Some(family_name) = face.name() else {
        return Err(PbfFontError::MissingFontFamilyName);
    };

    let mut stack = Fontstack {
        name: family_name,
        range: format!("{start}-{end}"),
        glyphs: Vec::with_capacity((end - start) as usize),
    };

    let mut renderer = FontRenderer::new();
    for char_code in start..=end {
        match render_sdf_glyph(&mut renderer, face, char_code, size, 3, radius, cutoff) {
            Ok(glyph) => {
                stack.glyphs.push(glyph);
            }
            Err(PbfFontError::SdfGlyphError(sdf_glyph_renderer::SdfGlyphError::MissingGlyph(
                _,
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
    let data: Arc<[u8]> = std::fs::read(font_path)?.into();
    let num_faces = FontFace::count(data.as_ref())?;

    let mut result = Glyphs::default();
    result.stacks.reserve(num_faces);

    for face_index in 0..num_faces {
        let face = FontFace::from_bytes(Arc::clone(&data), face_index)?;
        let stack = glyph_range_for_face(&face, start, end, size, radius, cutoff)?;
        result.stacks.push(stack);
    }

    Ok(result)
}
