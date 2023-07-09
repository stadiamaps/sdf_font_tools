use thiserror::Error;

#[derive(Debug, Error)]
pub enum SdfGlyphError {
    #[error("Missing size metrics")]
    MissingSizeMetrics,

    #[error("Invalid bitmap dimensions: The data length must be equal to {0} = {1}, but is equal to {2}.")]
    InvalidDataDimensions(&'static str, usize, usize),

    #[error("Cutoff values must be between 0 and 1 (both non-inclusive), but {0} was provided.")]
    InvalidCutoff(f64),

    #[cfg(feature = "freetype")]
    #[error("FreeType error: {0}")]
    FreeTypeError(#[from] freetype::Error),
}
