/// The rendered SDF glyph with its associated metrics.
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
