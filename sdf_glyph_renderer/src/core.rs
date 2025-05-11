use crate::SdfGlyphError;

/// A raw bitmap containing only the alpha channel.
#[derive(Debug, PartialEq, Eq)]
pub struct BitmapGlyph {
    /// The rendered, buffered glyph bitmap, flattened into a 1D array consisting only of only the
    /// alpha channel.
    pub(crate) alpha: Vec<u8>,

    /// The unbuffered width of the glyph in px.
    pub(crate) width: usize,

    /// The unbuffered height of the glyph in px.
    pub(crate) height: usize,

    /// The number of pixels buffering the glyph on all sides. You can add a buffer to a
    /// raw bitmap using the [`Self::from_unbuffered()`] constructor.
    pub(crate) buffer: usize,
}

impl BitmapGlyph {
    /// Creates a new bitmap from scratch.
    ///
    /// This constructor is useful when you already have a buffered glyph. If you have the alpha
    /// bitmap but still need to buffer it (what you expect from most font renderers, for example),
    /// use [`Self::from_unbuffered()`] instead.
    ///
    /// The dimensions provided is expected to describe the input data.
    pub fn new(
        alpha: Vec<u8>,
        width: usize,
        height: usize,
        buffer: usize,
    ) -> Result<BitmapGlyph, SdfGlyphError> {
        let expected = (width + buffer * 2) * (height + buffer * 2);
        if alpha.len() != expected {
            return Err(SdfGlyphError::InvalidDataDimensions(
                "(width + buffer * 2) * (height + buffer * 2)",
                expected,
                alpha.len(),
            ));
        }

        Ok(BitmapGlyph {
            alpha,
            width,
            height,
            buffer,
        })
    }

    /// Creates a new bitmap from a raw data source, buffered by a given amount.
    ///
    /// Most SDF glyphs are buffered a bit so that the outer edges can be properly captured.
    /// This constructor does the buffering for you.
    ///
    /// The dimensions provided is expected to describe the input data.
    pub fn from_unbuffered(
        alpha: &[u8],
        width: usize,
        height: usize,
        buffer: usize,
    ) -> Result<BitmapGlyph, SdfGlyphError> {
        let expected = width * height;
        if alpha.len() != expected {
            return Err(SdfGlyphError::InvalidDataDimensions(
                "width * height",
                expected,
                alpha.len(),
            ));
        }

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

        Ok(BitmapGlyph {
            alpha: buffered_data,
            width,
            height,
            buffer,
        })
    }

    /// Render a signed distance field for the given bitmap, recording distances
    /// out to `radius` pixels from the shape outline (the rest will be clamped).
    /// The range of the output field is [-1.0, 1.0], normalised to units of `radius`.
    #[must_use]
    pub fn render_sdf(&self, radius: usize) -> Vec<f64> {
        // Create two bitmaps, one for the pixels outside the filled area, and another for
        // values inside it.
        let mut outer_df: Vec<f64> = self
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

        let mut inner_df: Vec<f64> = self
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

        let buffered_width = self.width + self.buffer + self.buffer;
        let buffered_height = self.height + self.buffer + self.buffer;

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
}

/// An O(n) Euclidean Distance Transform algorithm.
/// See page 6 (420) of [paper](http://cs.brown.edu/people/pfelzens/papers/dt-final.pdf) for details and
/// further discussion of the math behind this.
fn dt(grid: &mut [f64], offset: usize, step_by: usize, size: usize) {
    // For our purposes, f is a one-dimensional slice of the grid
    let mut f = vec![0.0; size];
    let mut src = offset;
    for dst in 0..size {
        f[dst] = grid[src];
        src += step_by;
    }

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

/// Compresses a `Vec<f64>` into a `Vec<u8>` for efficiency.
///
/// The highest `cutoff` percent of values in the range (0-255) will be used to encode
/// negative values (points inside the glyph). This can be tuned based on the intended
/// application.
///
/// The `cutoff` value must be in the range (0, 1) - non-inclusive on both sides.
/// Values outside this range make no sense and will result in an error.
pub fn clamp_to_u8(sdf: &[f64], cutoff: f64) -> Result<Vec<u8>, SdfGlyphError> {
    if cutoff <= 0.0 || cutoff >= 1.0 {
        return Err(SdfGlyphError::InvalidCutoff(cutoff));
    }
    Ok(sdf
        .iter()
        .map(|v| {
            // Map the values back into the single byte integer range.
            // Note: casting from a float to an integer performs a saturating
            // cast in Rust, removing the need for special logic.
            // See https://doc.rust-lang.org/nomicon/casts.html.
            (255.0 - 255.0 * (v + cutoff)) as u8
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::{clamp_to_u8, BitmapGlyph};

    #[test]
    fn test_empty_glyph_unbuffered() {
        // Tests an empty glyph. In this case, we are using the actual bitmap (empty) and metrics
        // for how Open Sans Light encodes a space (0x20).
        let data = Vec::new();
        let bitmap = BitmapGlyph::new(data.clone(), 0, 0, 0).unwrap();
        let sdf = bitmap.render_sdf(8);

        assert_eq!(sdf, Vec::new());
        assert_eq!(clamp_to_u8(&sdf, 0.25).unwrap(), data);
    }

    #[test]
    fn test_empty_glyph_buffered() {
        // Tests an empty glyph. In this case, we are using the actual bitmap (empty) and metrics
        // for how Open Sans Light encodes a space (0x20), plus a 3px buffer we added.
        let data = Vec::from([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0,
        ]);
        let bitmap = BitmapGlyph::new(data.clone(), 0, 0, 3).unwrap();
        let sdf = bitmap.render_sdf(8);
        let sdf_data_f64 = Vec::from([
            1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
            1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
            1.0, 1.0,
        ]);

        assert_eq!(sdf, sdf_data_f64);
        assert_eq!(clamp_to_u8(&sdf, 0.25).unwrap(), data);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    fn test_nontrivial_glyph() {
        // Tests an nontrivial glyph. In this case, we are using the actual bitmap and metrics
        // for how Open Sans Light encodes an ampersand (0x25), plus a 3px buffer we added.
        let alpha = Vec::from(include!("../fixtures/glyph_alpha.json"));
        let sdf_data_f64 = Vec::from(include!("../fixtures/glyph_sdf_f64.json"));
        let sdf_data_u8 = Vec::from(include!("../fixtures/glyph_sdf_u8.json"));
        let bitmap = BitmapGlyph::new(alpha, 16, 19, 3).unwrap();
        let sdf = bitmap.render_sdf(8);

        assert_eq!(sdf, sdf_data_f64);
        assert_eq!(clamp_to_u8(&sdf, 0.25).unwrap(), sdf_data_u8);

        // Sanity check on the clamp_to_u8 function; 191 = 255 (max value of a u8) - 25% of 256 (range)
        assert_eq!(
            clamp_to_u8(&sdf, 0.25)
                .unwrap()
                .into_iter()
                .filter(|x| *x >= 191)
                .count(),
            sdf_data_f64.into_iter().filter(|x| *x < 0.0).count()
        );
    }
}
