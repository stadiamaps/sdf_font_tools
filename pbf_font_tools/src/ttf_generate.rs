use std::path::Path;

use sdf_glyph_renderer::ttf_parser;
use sdf_glyph_renderer::{clamp_to_u8, render_sdf_from_ttf};

use crate::error::PbfFontError;
use crate::{Fontstack, Glyph, Glyphs};

/// Renders a single glyph for the given ttf-parser face into a Glyph message.
pub fn render_sdf_glyph_ttf(
    face: &ttf_parser::Face<'_>,
    char_code: u32,
    font_size: f64,
    buffer: usize,
    radius: usize,
    cutoff: f64,
) -> Result<Glyph, PbfFontError> {
    let glyph = render_sdf_from_ttf(face, char_code, font_size, buffer, radius)?;

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

/// Renders a glyph range for the given ttf-parser face into a Mapbox-compatible fontstack.
///
/// This is the ttf-parser equivalent of [`glyph_range_for_face`](crate::glyph_range_for_face).
///
/// The `radius` and `cutoff` parameters are exposed in case you are working with an
/// alternate renderer with tunable options, but you are probably best off sticking
/// with 8 and 0.25 respectively.
pub fn glyph_range_for_face_ttf(
    face: &ttf_parser::Face<'_>,
    start: u32,
    end: u32,
    size: f64,
    radius: usize,
    cutoff: f64,
) -> Result<Fontstack, PbfFontError> {
    // Extract font family name from the name table
    let family_name = face
        .names()
        .into_iter()
        .filter(|name: &ttf_parser::name::Name| name.name_id == ttf_parser::name_id::FULL_NAME)
        .find_map(|name: ttf_parser::name::Name| name.to_string())
        .or_else(|| {
            face.names()
                .into_iter()
                .filter(|name: &ttf_parser::name::Name| {
                    name.name_id == ttf_parser::name_id::FAMILY
                })
                .find_map(|name: ttf_parser::name::Name| name.to_string())
        })
        .ok_or(PbfFontError::MissingFontFamilyName)?;

    let mut stack = Fontstack {
        name: family_name,
        range: format!("{start}-{end}"),
        glyphs: Vec::with_capacity((end - start) as usize),
    };

    for char_code in start..=end {
        match render_sdf_glyph_ttf(face, char_code, size, 3, radius, cutoff) {
            Ok(glyph) => {
                stack.glyphs.push(glyph);
            }
            Err(PbfFontError::SdfGlyphError(
                sdf_glyph_renderer::SdfGlyphError::GlyphNotFound(_),
            ))
            | Err(PbfFontError::SdfGlyphError(
                sdf_glyph_renderer::SdfGlyphError::InvalidCharCode(_),
            )) => {
                // Do nothing; not all glyphs will be present in a font.
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    Ok(stack)
}

/// Renders glyph ranges for all faces in a font file using ttf-parser.
pub fn glyph_range_for_font_ttf<P: AsRef<Path>>(
    font_path: P,
    start: u32,
    end: u32,
    size: f64,
    radius: usize,
    cutoff: f64,
) -> Result<Glyphs, PbfFontError> {
    let font_data = std::fs::read(font_path.as_ref())?;
    let num_faces = ttf_parser::fonts_in_collection(&font_data).unwrap_or(1);

    let mut result = Glyphs::default();
    result.stacks.reserve(num_faces as usize);

    for face_index in 0..num_faces {
        let face = ttf_parser::Face::parse(&font_data, face_index)
            .map_err(|e: ttf_parser::FaceParsingError| {
                PbfFontError::TtfParserError(e.to_string())
            })?;

        let stack = glyph_range_for_face_ttf(&face, start, end, size, radius, cutoff)?;
        result.stacks.push(stack);
    }

    Ok(result)
}
