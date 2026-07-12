#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageFacts {
    pub media_type: String,
    pub width: u32,
    pub height: u32,
    pub bit_depth: u8,
    pub has_alpha: bool,
}
pub trait ImageInspector {
    fn inspect(&self, bytes: &[u8]) -> Result<ImageFacts, String>;
}
