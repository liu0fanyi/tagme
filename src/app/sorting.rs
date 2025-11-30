use crate::app::types::{FileListItem, FileInfo, DisplayFile, TagInfo, SortColumn, SortDirection};
use std::collections::{HashMap, HashSet};

pub fn build_display_files(
    scanned: &[FileListItem],
    db: &[FileInfo],
    tags_map: &HashMap<u32, Vec<TagInfo>>,
    selected_tag_ids: &[u32],
) -> Vec<DisplayFile> {
    let mut display_files: Vec<DisplayFile> = Vec::new();
    let mut seen_paths: HashSet<String> = HashSet::new();
    for file in db {
        let path_obj = std::path::Path::new(&file.path);
        let name = path_obj.file_name().unwrap_or_default().to_string_lossy().to_string();
        let extension = path_obj.extension().unwrap_or_default().to_string_lossy().to_string();
        seen_paths.insert(file.path.clone());
        display_files.push(DisplayFile {
            path: file.path.clone(),
            name,
            extension,
            size_bytes: file.size_bytes,
            last_modified: file.last_modified,
            db_id: Some(file.id),
            tags: tags_map.get(&file.id).cloned().unwrap_or_default(),
            is_directory: file.is_directory,
        });
    }
    let has_tag_filter = !selected_tag_ids.is_empty();
    if !has_tag_filter {
        for file in scanned {
            if !seen_paths.contains(&file.path) {
                let path_obj = std::path::Path::new(&file.path);
                let name = path_obj.file_name().unwrap_or_default().to_string_lossy().to_string();
                let extension = path_obj.extension().unwrap_or_default().to_string_lossy().to_string();
                display_files.push(DisplayFile {
                    path: file.path.clone(),
                    name,
                    extension,
                    size_bytes: file.size_bytes,
                    last_modified: file.last_modified,
                    db_id: None,
                    tags: Vec::new(),
                    is_directory: file.is_directory,
                });
            }
        }
    }
    display_files
}

pub fn sort_display_files(mut display_files: Vec<DisplayFile>, col: SortColumn, dir: SortDirection) -> Vec<DisplayFile> {
    display_files.sort_by(|a, b| {
        let cmp = match col {
            SortColumn::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            SortColumn::Size => a.size_bytes.cmp(&b.size_bytes),
            SortColumn::Date => a.last_modified.cmp(&b.last_modified),
            SortColumn::Type => a.extension.to_lowercase().cmp(&b.extension.to_lowercase()),
        };
        match dir { SortDirection::Asc => cmp, SortDirection::Desc => cmp.reverse() }
    });
    display_files
}
