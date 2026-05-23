use std::sync::Arc;

use swash::scale::{Render, ScaleContext, Source};
use swash::{CacheKey, FontDataRef, FontRef, StringId};

use crate::{BitmapGlyph, SdfGlyphError};

pub struct FontFace {
    data: Arc<[u8]>,
    offset: u32,
    key: CacheKey,
}

impl FontFace {
    pub fn from_bytes(data: Arc<[u8]>, index: usize) -> Result<Self, SdfGlyphError> {
        let font =
            FontRef::from_index(data.as_ref(), index).ok_or(SdfGlyphError::InvalidFontData)?;
        let offset = font.offset;
        let key = font.key;
        Ok(Self { data, offset, key })
    }

    pub fn count(data: &[u8]) -> Result<usize, SdfGlyphError> {
        FontDataRef::new(data)
            .map(|font_data| font_data.len())
            .ok_or(SdfGlyphError::InvalidFontData)
    }

    pub fn name(&self) -> Option<String> {
        let font = self.as_ref();
        let strings = font.localized_strings();
        strings
            .find_by_id(StringId::Full, None)
            .map(|name| name.to_string())
            .filter(|name| !name.is_empty())
            .or_else(|| {
                let family = strings.find_by_id(StringId::Family, None)?.to_string();
                if family.is_empty() {
                    return None;
                }

                let subfamily = strings
                    .find_by_id(StringId::SubFamily, None)
                    .map(|name| name.to_string())
                    .filter(|name| !name.is_empty());

                Some(match subfamily {
                    Some(subfamily) => format!("{family} {subfamily}"),
                    None => family,
                })
            })
    }

    fn as_ref(&self) -> FontRef<'_> {
        FontRef {
            data: self.data.as_ref(),
            offset: self.offset,
            key: self.key,
        }
    }
}

pub struct FontRenderer {
    context: ScaleContext,
}

impl FontRenderer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            context: ScaleContext::new(),
        }
    }

    pub fn render_sdf_from_face(
        &mut self,
        face: &FontFace,
        char_code: u32,
        size: f32,
        buffer: usize,
        radius: usize,
    ) -> Result<SdfGlyph, SdfGlyphError> {
        render_sdf_from_face_with_context(&mut self.context, face, char_code, size, buffer, radius)
    }
}

impl Default for FontRenderer {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SdfGlyph {
    pub sdf: Vec<f64>,
    pub metrics: GlyphMetrics,
}

/// Metrics used to position the rendered glyph bitmap relative to the baseline.
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

/// Renders a glyph from a font face and generates an SDF using the font's embedded metrics.
pub fn render_sdf_from_face(
    face: &FontFace,
    char_code: u32,
    size: f32,
    buffer: usize,
    radius: usize,
) -> Result<SdfGlyph, SdfGlyphError> {
    let mut renderer = FontRenderer::new();
    renderer.render_sdf_from_face(face, char_code, size, buffer, radius)
}

fn render_sdf_from_face_with_context(
    context: &mut ScaleContext,
    face: &FontFace,
    char_code: u32,
    size: f32,
    buffer: usize,
    radius: usize,
) -> Result<SdfGlyph, SdfGlyphError> {
    let Some(_) = char::from_u32(char_code) else {
        return Err(SdfGlyphError::MissingGlyph(char_code));
    };

    let font = face.as_ref();
    let glyph_id = font.charmap().map(char_code);
    if glyph_id == 0 {
        return Err(SdfGlyphError::MissingGlyph(char_code));
    }

    let font_metrics = font.metrics(&[]).scale(size);
    let glyph_metrics = font.glyph_metrics(&[]).scale(size);
    let advance_width = glyph_metrics.advance_width(glyph_id).round().max(0.0) as u32;

    let mut scaler = context.builder(font).size(size).hint(false).build();
    let image = Render::new(&[Source::Outline]).render(&mut scaler, glyph_id);

    let (alpha, width, height, left_bearing, top_bearing) = if let Some(image) = image {
        (
            image.data,
            image.placement.width as usize,
            image.placement.height as usize,
            image.placement.left,
            image.placement.top,
        )
    } else {
        (Vec::new(), 0, 0, 0, 0)
    };

    let bitmap = BitmapGlyph::from_unbuffered(&alpha, width, height, buffer)?;
    let metrics = GlyphMetrics {
        width: bitmap.width,
        height: bitmap.height,
        left_bearing,
        top_bearing,
        h_advance: advance_width,
        ascender: font_metrics.ascent.round() as i32,
    };

    Ok(SdfGlyph {
        sdf: bitmap.render_sdf(radius),
        metrics,
    })
}
