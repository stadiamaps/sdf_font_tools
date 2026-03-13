#[derive(thiserror::Error, Debug)]
pub enum PbfFontError {
    #[error("Sub-process error: {0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("Protobuf decoding error: {0}")]
    ProtobufError(#[from] prost::DecodeError),
    #[cfg(any(feature = "freetype", feature = "ttf-parser"))]
    #[error("SDF glyph error: {0}")]
    SdfGlyphError(#[from] sdf_glyph_renderer::SdfGlyphError),
    #[error("Font family name is not set")]
    MissingFontFamilyName,
    #[cfg(feature = "freetype")]
    #[error("Freetype error: {0}")]
    FreetypeError(#[from] crate::freetype::Error),
    #[error("ttf-parser error: {0}")]
    TtfParserError(String),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}
