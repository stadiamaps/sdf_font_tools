//! This crate is a Rust implementation of the signed distance field generation techniques
//! demonstrated by [Valve](https://steamcdn-a.akamaihd.net/apps/valve/2007/SIGGRAPH2007_AlphaTestedMagnification.pdf)
//! and [Mapbox](https://blog.mapbox.com/drawing-text-with-signed-distance-fields-in-mapbox-gl-b0933af6f817).
//! The generic interface works with any bitmap, and a high level interface enables easy operation
//! with FreeType faces when the optional `freetype` feature is enabled.
//!
//! The approach taken by this crate is similar to [TinySDF](https://github.com/mapbox/tiny-sdf);
//! it works from a raster bitmap rather than directly from vector outlines. This keeps the
//! crate simple and allows it to be used generically with any bitmap. The SDF is calculated
//! using the same algorithm described in [this paper](http://cs.brown.edu/people/pfelzens/papers/dt-final.pdf)
//! by Felzenszwalb & Huttenlocher.
//!
//! Rather than re-invent the rasterisation process for fonts, this crate relies on FreeType to
//! generate the bitmap. This is quite fast (we're talking µs/glyph), and the results are
//! almost always indistinguishable from the more sophisticated vector-based approach of
//! [sdf-glyph-foundry](https://github.com/mapbox/sdf-glyph-foundry).
//!
//! This crate is used by [pbf_font_tools](https://github.com/stadiamaps/pbf_font_tools) to generate
//! SDF glyphs from any FreeType-readable font. If you're looking for a batch generation tool,
//! check out [build_pbf_glyphs](https://github.com/stadiamaps/build_pbf_glyphs).

#[cfg(test)]
mod tests;

#[cfg(feature = "freetype")]
use freetype::{face::LoadFlag, Face};

#[macro_use]
extern crate derive_error;

#[derive(Debug, Error)]
pub enum Error {
    #[cfg(feature = "freetype")]
    FreeTypeError(freetype::Error),
    MissingSizeMetrics,
}

#[cfg(feature = "freetype")]
pub struct SDFGlyph {
    pub sdf: Vec<f64>,
    pub metrics: GlyphMetrics,
}

/// For an explanation of the technical terms used when describing the glyph metrics,
/// the [FreeType tutorial](https://www.freetype.org/freetype2/docs/tutorial/step2.html) is a
/// fantastic reference.
#[cfg(feature = "freetype")]
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

/// A raw bitmap containing only the alpha channel.
#[derive(Debug, PartialEq, Eq)]
pub struct BitmapGlyph {
    /// The rendered, buffered glyph bitmap, flattened into a 1D array consisting only of only the
    /// alpha channel.
    alpha: Vec<u8>,

    /// The unbuffered width of the glyph in px.
    width: usize,

    /// The unbuffered height of the glyph in px.
    height: usize,

    /// The number of pixels buffering the glyph on all sides. You can add a buffer to a
    /// raw bitmap using the [`Self::from_unbuffered()`] constructor.
    buffer: usize,
}

impl BitmapGlyph {
    /// Creates a new bitmap from scratch.
    ///
    /// This constructor is useful when you already have a buffered glyph. If you have the alpha
    /// bitmap but still need to buffer it (what you expect from most font renderers, for example),
    /// use [`Self::from_unbuffered()`] instead.
    ///
    /// # Panics
    ///
    /// The caller is responsible for checking the integrity of the data. If the dimensions provided
    /// can't describe the input data, this function will panic.
    pub fn new(alpha: Vec<u8>, width: usize, height: usize, buffer: usize) -> BitmapGlyph {
        assert_eq!(
            alpha.len(),
            (width + buffer * 2) * (height + buffer * 2),
            "The data length must be equal to (width + buffer * 2) * (height + buffer * 2)."
        );

        BitmapGlyph {
            alpha,
            width,
            height,
            buffer,
        }
    }

    /// Creates a new bitmap from a raw data source, buffered by a given amount.
    ///
    /// Most SDF glyphs are buffered a bit so that the outer edges can be properly captured.
    /// This constructor does the buffering for you.
    ///
    /// # Panics
    ///
    /// The caller is responsible for checking the integrity of the data. If the dimensions provided
    /// can't describe the input data, this function will panic.
    pub fn from_unbuffered(
        alpha: &[u8],
        width: usize,
        height: usize,
        buffer: usize,
    ) -> BitmapGlyph {
        assert_eq!(
            alpha.len(),
            width * height,
            "The data length must be equal to width * height."
        );

        let double_buffer = buffer + buffer;

        // Create an larger bitmap that includes a buffer
        let mut buffered_data = vec![0u8; (width + double_buffer) * (height + double_buffer)];

        // Copy the bitmap
        for x in 0..width {
            for y in 0..height {
                buffered_data[(y + buffer) * (width + double_buffer) + x + buffer] =
                    alpha[y * width + x];
            }
        }

        BitmapGlyph {
            alpha: buffered_data,
            width,
            height,
            buffer,
        }
    }
}

/// An O(n) Euclidean Distance Transform algorithm.
/// See page 6 (420) of http://cs.brown.edu/people/pfelzens/papers/dt-final.pdf for details and
/// further discussion of the math behind this.
fn dt(grid: &mut [f64], offset: usize, step_by: usize, size: usize) {
    // For our purposes, f is a one-dimensional slice of the grid
    let f: Vec<f64> = grid.iter().skip(offset).step_by(step_by).copied().collect();

    // It may be possible to make this more functional in style,
    // but for now this is more or less a "dumb" transcription of
    // the algorithm presented in the paper by Felzenszwalb & Huttenlocher.
    let mut k = 0;
    let mut v = vec![0; size];
    let mut z = vec![0f64; size + 1];
    let mut s: f64;

    z[0] = f64::MIN;
    z[1] = f64::MAX;

    for q in 1..size {
        loop {
            let q2 = (q * q) as f64;
            let vk2 = (v[k] * v[k]) as f64;
            let denom = (2 * q - 2 * v[k]) as f64;
            s = ((f[q] + q2) - (f[v[k]] + vk2)) / denom;

            if s <= z[k] {
                k -= 1;
            } else {
                k += 1;
                v[k] = q;
                z[k] = s;
                z[k + 1] = f64::MAX;

                break;
            }
        }
    }

    k = 0;
    for q in 0..size {
        let qf64 = q as f64;
        while z[k + 1] < qf64 {
            k += 1;
        }
        let vkf64 = v[k] as f64;
        grid[offset + q * step_by] = (qf64 - vkf64) * (qf64 - vkf64) + f[v[k]];
    }
}

/// Render a signed distance field for the given bitmap, recording distances
/// out to `radius` pixels from the shape outline (the rest will be clamped).
/// The range of the output field is [-1.0, 1.0], normalised to units of `radius`.
pub fn render_sdf(bitmap: &BitmapGlyph, radius: usize) -> Vec<f64> {
    // Create two bitmaps, one for the pixels outside the filled area, and another for
    // values inside it.
    let mut outer_df: Vec<f64> = bitmap
        .alpha
        .iter()
        .map(|alpha| {
            if *alpha == 0 {
                f64::MAX // Perfectly outside the shape
            } else {
                // Values with alpha < 50% will be treated as progressively
                // further "outside" the shape as their alpha decreases. Alpha > 50%
                // will be treated as "inside" the shape and get a zero value here.
                0f64.max(0.5 - (*alpha as f64 / 255.0)).powi(2)
            }
        })
        .collect();
    let mut inner_df: Vec<f64> = bitmap
        .alpha
        .iter()
        .map(|alpha| {
            if *alpha == 255 {
                f64::MAX // Perfectly inside the shape
            } else {
                // Values with alpha > 50% will be treated as progressively
                // further "inside" the shape as their alpha decreases. Alpha < 50%
                // will be treated as "outside" the shape and get a zero value here.
                0f64.max((*alpha as f64 / 255.0) - 0.5).powi(2)
            }
        })
        .collect();

    let buffered_width = bitmap.width + bitmap.buffer + bitmap.buffer;
    let buffered_height = bitmap.height + bitmap.buffer + bitmap.buffer;

    // Per page 8 (422), the 2D distance transform can be obtained by computing the
    // one-dimensional distance transform along each column first and then computing
    // the transform along each row of the result. We run the transform over both the
    // outer and inner to get the respective Euclidean squared distances
    // (the math is much easier this way).
    for col in 0..buffered_width {
        dt(&mut outer_df, col, buffered_width, buffered_height);
        dt(&mut inner_df, col, buffered_width, buffered_height);
    }

    for row in 0..buffered_height {
        dt(&mut outer_df, row * buffered_width, 1, buffered_width);
        dt(&mut inner_df, row * buffered_width, 1, buffered_width);
    }

    outer_df
        .iter()
        .zip(inner_df.iter())
        .map(|(outer_df, inner_df)| {
            // Determine the euclidean distance inside or outside the alpha mask, then
            // clamp the range according to the radius so that the overall range of the
            // output field is [-1, 1] as a percentage of the radius.
            ((outer_df.sqrt() - inner_df.sqrt()) / radius as f64)
                .min(1.0)
                .max(-1.0)
        })
        .collect()
}

/// Compresses a Vec<f64> into a Vec<u8> for efficiency.
///
/// The highest `cutoff` percent of values in the range (0-255) will be used to encode
/// negative values (points inside the glyph). This can be tuned based on the intended
/// application.
///
/// # Panics
///
/// The `cutoff` value must be in the range (0, 1). Values outside this range make no sense and
/// will cause a panic.
pub fn clamp_to_u8(sdf: &[f64], cutoff: f64) -> Vec<u8> {
    assert!(
        cutoff > 0.0 && cutoff < 1.0,
        "Cutoff values must be between 0 and 1"
    );
    sdf.iter()
        .map(|v| {
            // Map the values back into the single byte integer range.
            // Note: casting from a float to an integer performs a saturating
            // cast in Rust, removing the need for special logic.
            // See https://doc.rust-lang.org/nomicon/casts.html.
            (255.0 - 255.0 * (v + cutoff)) as u8
        })
        .collect()
}

/// This is a convenient frontend to [`render_sdf`](fn@render_sdf) that accepts a FreeType
/// face as input and generates bitmaps automatically using the font's embedded metrics.
#[cfg(feature = "freetype")]
pub fn render_sdf_from_face(
    face: &Face,
    char_code: u32,
    buffer: usize,
    radius: usize,
) -> Result<SDFGlyph, Error> {
    let ascender = (face
        .size_metrics()
        .ok_or(Error::MissingSizeMetrics)?
        .ascender
        >> 6) as i32;

    let glyph_index = face.get_char_index(char_code as usize);
    if glyph_index == 0 {
        // TODO: PR to freetype-rs since this is not raised as an error? Seems like it should be.
        return Err(Error::FreeTypeError(freetype::Error::InvalidGlyphIndex));
    }

    face.load_glyph(glyph_index, LoadFlag::NO_HINTING | LoadFlag::RENDER)?;

    let glyph = face.glyph();
    let glyph_bitmap = glyph.bitmap();
    let bitmap = BitmapGlyph::from_unbuffered(
        glyph_bitmap.buffer(),
        glyph_bitmap.width() as usize,
        glyph_bitmap.rows() as usize,
        buffer,
    );
    let metrics = GlyphMetrics {
        width: bitmap.width,
        height: bitmap.height,
        left_bearing: glyph.bitmap_left(),
        top_bearing: glyph.bitmap_top(),
        h_advance: (glyph.metrics().horiAdvance >> 6) as u32,
        ascender,
    };

    Ok(SDFGlyph {
        sdf: render_sdf(&bitmap, radius),
        metrics,
    })
}
