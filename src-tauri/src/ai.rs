#[cfg(feature = "ai")]
pub fn recommend_by_title_candle(title: &str, tag_names: &[String]) -> Option<Vec<(String, f32)>> {
    None
}

#[cfg(not(feature = "ai"))]
pub fn recommend_by_title_candle(_title: &str, _tag_names: &[String]) -> Option<Vec<(String, f32)>> {
    None
}
