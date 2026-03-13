use freetype::face::LoadFlag;
use freetype::Face;

use crate::{BitmapGlyph, GlyphMetrics, SdfGlyph, SdfGlyphError};

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

    let Some(glyph_index) = face.get_char_index(char_code as usize) else {
        return Err(SdfGlyphError::FreeTypeError(
            freetype::Error::InvalidGlyphIndex,
        ));
    };

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
