use ttf_parser::Face;

use crate::outline::{GlyphOutline, OutlineBuilder};
use crate::{GlyphMetrics, SdfGlyph, SdfGlyphError};

/// Extract the glyph outline from a ttf-parser face for a given character.
///
/// The outline is transformed to pixel coordinates based on the given `font_size` (in px).
/// The glyph is positioned with its bounding box starting at `(buffer, buffer)` in the output grid.
/// Font Y-up coordinates are flipped to grid Y-down coordinates.
fn extract_outline(
    face: &Face<'_>,
    glyph_id: ttf_parser::GlyphId,
    font_size: f64,
    buffer: usize,
) -> Result<(GlyphOutline, GlyphMetrics), SdfGlyphError> {
    let units_per_em = face.units_per_em() as f64;
    let scale = font_size / units_per_em;

    let ascender = (face.ascender() as f64 * scale).round() as i32;

    // Get glyph bounding box for dimensions
    let bbox = face
        .glyph_bounding_box(glyph_id)
        .unwrap_or(ttf_parser::Rect {
            x_min: 0,
            y_min: 0,
            x_max: 0,
            y_max: 0,
        });

    let glyph_width = ((bbox.x_max as f64 - bbox.x_min as f64) * scale).round() as usize;
    let glyph_height = ((bbox.y_max as f64 - bbox.y_min as f64) * scale).round() as usize;

    let h_advance = face
        .glyph_hor_advance(glyph_id)
        .map(|a| (a as f64 * scale).round() as u32)
        .unwrap_or(0);

    let left_bearing = (bbox.x_min as f64 * scale).round() as i32;
    let top_bearing = (bbox.y_max as f64 * scale).round() as i32;

    let metrics = GlyphMetrics {
        width: glyph_width,
        height: glyph_height,
        left_bearing,
        top_bearing,
        h_advance,
        ascender,
    };

    // Pre-compute the transform from font units to pixel coordinates.
    // X: scale + offset to place glyph bbox at (buffer, ...)
    // Y: flip (negate scale) + offset to place glyph bbox at (..., buffer)
    let offset_x = buffer as f64 - bbox.x_min as f64 * scale;
    let offset_y = buffer as f64 + bbox.y_max as f64 * scale;

    // Build the outline by pre-transforming coordinates before passing to OutlineBuilder.
    // We set builder scale=1 and offset=0 since we handle the transform ourselves
    // (to support the Y-flip which OutlineBuilder doesn't natively handle).
    let mut builder = OutlineBuilder::new(1.0, 0.0, 0.0);

    struct FlipAdapter<'a> {
        scale: f64,
        offset_x: f64,
        offset_y: f64,
        builder: &'a mut OutlineBuilder,
    }

    impl FlipAdapter<'_> {
        fn tx(&self, x: f64, y: f64) -> (f64, f64) {
            (
                x * self.scale + self.offset_x,
                -y * self.scale + self.offset_y,
            )
        }
    }

    impl ttf_parser::OutlineBuilder for FlipAdapter<'_> {
        fn move_to(&mut self, x: f32, y: f32) {
            let (px, py) = self.tx(x as f64, y as f64);
            self.builder.move_to(px, py);
        }

        fn line_to(&mut self, x: f32, y: f32) {
            let (px, py) = self.tx(x as f64, y as f64);
            self.builder.line_to(px, py);
        }

        fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
            let (px1, py1) = self.tx(x1 as f64, y1 as f64);
            let (px, py) = self.tx(x as f64, y as f64);
            self.builder.quad_to(px1, py1, px, py);
        }

        fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
            let (px1, py1) = self.tx(x1 as f64, y1 as f64);
            let (px2, py2) = self.tx(x2 as f64, y2 as f64);
            let (px, py) = self.tx(x as f64, y as f64);
            self.builder.curve_to(px1, py1, px2, py2, px, py);
        }

        fn close(&mut self) {
            // Handled by OutlineBuilder on move_to/finish
        }
    }

    let mut adapter = FlipAdapter {
        scale,
        offset_x,
        offset_y,
        builder: &mut builder,
    };

    face.outline_glyph(glyph_id, &mut adapter);
    drop(adapter);

    let outline = builder.finish();

    Ok((outline, metrics))
}

/// Render an SDF glyph from a ttf-parser face for a given character code.
///
/// This is the ttf-parser equivalent of [`render_sdf_from_face`](crate::render_sdf_from_face)
/// (the FreeType-based function).
///
/// - `font_size`: the desired font size in pixels (typically 24 for MapLibre compatibility)
/// - `buffer`: padding around the glyph in pixels (typically 3)
/// - `radius`: SDF distance radius in pixels (typically 8)
pub fn render_sdf_from_ttf(
    face: &Face<'_>,
    char_code: u32,
    font_size: f64,
    buffer: usize,
    radius: usize,
) -> Result<SdfGlyph, SdfGlyphError> {
    let c = char::from_u32(char_code).ok_or(SdfGlyphError::InvalidCharCode(char_code))?;

    let glyph_id = face
        .glyph_index(c)
        .ok_or(SdfGlyphError::GlyphNotFound(char_code))?;

    let (outline, metrics) = extract_outline(face, glyph_id, font_size, buffer)?;

    let sdf = outline.render_sdf(metrics.width, metrics.height, buffer, radius);

    Ok(SdfGlyph { sdf, metrics })
}
