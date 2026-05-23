#[derive(thiserror::Error, Debug)]
pub enum PbfFontError {
    #[error("Sub-process error: {0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("Protobuf decoding error: {0}")]
    ProtobufError(#[from] prost::DecodeError),
    #[cfg(feature = "generate")]
    #[error("SDF glyph error: {0}")]
    SdfGlyphError(#[from] sdf_glyph_renderer::SdfGlyphError),
    #[error("Font family name is not set")]
    MissingFontFamilyName,
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}
