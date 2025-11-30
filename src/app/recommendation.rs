use crate::app::types::TagInfo;
use leptos_recommender::RecommendItem;

pub fn map_items_to_tags(items: &[RecommendItem], tags: &[TagInfo]) -> Vec<TagInfo> {
    let mut out: Vec<TagInfo> = Vec::new();
    for item in items {
        if let Some(t) = tags.iter().find(|x| x.name == item.name) { out.push(t.clone()); }
    }
    out
}
