use freetype::{face::LoadFlag, Face};

use crate::BitmapGlyph;
use crate::SdfGlyphError;

pub struct SdfGlyph {
    pub sdf: Vec<f64>,
    pub metrics: GlyphMetrics,
}

/// For an explanation of the technical terms used when describing the glyph metrics,
/// the [FreeType tutorial](https://www.freetype.org/freetype2/docs/tutorial/step2.html) is a
/// fantastic reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlyphMetrics {
    /// The unbuffered width of the glyph in px.
    pub width: usize,

    /// The unbuffered height of the glyph in px.
    pub height: usize,

    /// The left bearing of the glyph in px.
    pub left_bearing: i32,

    /// The top bearing of the glyph in px.
    pub top_bearing: i32,

    /// The horizontal advance of the glyph in px.
    ///
    /// Note: vertical advance is not currently tracked; this is something we may
    /// consider addressing in a future release, but most renderers, do not support vertical
    /// text layouts so this is not much of a priority at the moment.
    pub h_advance: u32,

    /// The typographical ascender in px.
    pub ascender: i32,
}

/// This is a convenient frontend to [`render_sdf`](BitmapGlyph::render_sdf) that accepts a FreeType
/// face as input and generates bitmaps automatically using the font's embedded metrics.
pub fn render_sdf_from_face(
    face: &Face,
    char_code: u32,
    buffer: usize,
    radius: usize,
) -> Result<SdfGlyph, SdfGlyphError> {
    let ascender = (face
        .size_metrics()
        .ok_or(SdfGlyphError::MissingSizeMetrics)?
        .ascender
        >> 6) as i32;

    let glyph_index = face.get_char_index(char_code as usize);
    if glyph_index == 0 {
        // See also https://github.com/PistonDevelopers/freetype-rs/pull/252
        return Err(SdfGlyphError::FreeTypeError(
            freetype::Error::InvalidGlyphIndex,
        ));
    }

    face.load_glyph(glyph_index, LoadFlag::NO_HINTING | LoadFlag::RENDER)?;

    let glyph = face.glyph();
    let glyph_bitmap = glyph.bitmap();
    let bitmap = BitmapGlyph::from_unbuffered(
        glyph_bitmap.buffer(),
        glyph_bitmap.width() as usize,
        glyph_bitmap.rows() as usize,
        buffer,
    )?;
    let metrics = GlyphMetrics {
        width: bitmap.width,
        height: bitmap.height,
        left_bearing: glyph.bitmap_left(),
        top_bearing: glyph.bitmap_top(),
        h_advance: (glyph.metrics().horiAdvance >> 6) as u32,
        ascender,
    };

    Ok(SdfGlyph {
        sdf: bitmap.render_sdf(radius),
        metrics,
    })
}
