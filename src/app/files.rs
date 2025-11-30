use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;
use serde::Serialize;
use crate::app::types::*;
use crate::app::api::invoke;

pub async fn load_tags(set_all_tags: WriteSignal<Vec<TagInfo>>) {
    web_sys::console::log_1(&"Loading tags...".into());
    let tags_val = invoke("get_all_tags", JsValue::NULL).await;

    match serde_wasm_bindgen::from_value::<Vec<TagInfo>>(tags_val) {
        Ok(tags) => {
            web_sys::console::log_1(&format!("Loaded {} tags", tags.len()).into());
            for tag in &tags {
                web_sys::console::log_1(&format!("   Frontend - Tag: {}, ID: {}, Parent: {:?}, Pos: {}",
                    tag.name, tag.id, tag.parent_id, tag.position).into());
            }
            set_all_tags.set(tags);
        },
        Err(e) => {
            web_sys::console::error_1(&format!("Error deserializing tags: {:?}", e).into());
        }
    }
}

pub async fn load_all_files(
    set_all_files: WriteSignal<Vec<FileInfo>>,
    set_displayed_files: WriteSignal<Vec<FileInfo>>,
    set_file_tags_map: WriteSignal<std::collections::HashMap<u32, Vec<TagInfo>>>,
) {
    let files_val = invoke("get_all_files", JsValue::NULL).await;
    let files = match serde_wasm_bindgen::from_value::<Vec<FileInfo>>(files_val) {
        Ok(f) => f,
        Err(e) => {
            web_sys::console::error_1(&format!("Error loading files: {:?}", e).into());
            return;
        }
    };
    
    // Load tags for each file
    let mut tags_map = std::collections::HashMap::new();
    for file in &files {
        let file_id = file.id;
        let args = GetFileTagsArgs { file_id };
        let tags_val = invoke("get_file_tags", serde_wasm_bindgen::to_value(&args).unwrap()).await;
        
        if let Ok(tags) = serde_wasm_bindgen::from_value::<Vec<TagInfo>>(tags_val) {
            tags_map.insert(file_id, tags);
        }
    }
    
    set_file_tags_map.set(tags_map);
    set_all_files.set(files.clone());
    set_displayed_files.set(files);
}

pub fn filter_files(
    tag_ids: Vec<u32>,
    use_and: bool,
    set_displayed_files: WriteSignal<Vec<FileInfo>>,
    all_files: Vec<FileInfo>,
) {
    if tag_ids.is_empty() {
        set_displayed_files.set(all_files);
        return;
    }

    spawn_local(async move {
        web_sys::console::log_1(&format!("filter_files start, tag_ids={:?}, use_and={}", tag_ids, use_and).into());
        let args = FilterFilesByTagsArgs {
            tag_ids,
            use_and_logic: use_and,
        };
        let result_val = invoke("filter_files_by_tags", serde_wasm_bindgen::to_value(&args).unwrap()).await;
        
        if let Ok(files) = serde_wasm_bindgen::from_value::<Vec<FileInfo>>(result_val) {
            web_sys::console::log_1(&format!("filter_files result count={}", files.len()).into());
            set_displayed_files.set(files);
        }
    });
}

pub fn handle_scan_directory(
    root_directories: ReadSignal<Vec<String>>,
    set_scanning: WriteSignal<bool>,
    set_scanned_files: WriteSignal<Vec<FileListItem>>,
    set_all_files: WriteSignal<Vec<FileInfo>>,
    set_displayed_files: WriteSignal<Vec<FileInfo>>,
    set_file_tags_map: WriteSignal<std::collections::HashMap<u32, Vec<TagInfo>>>,
) {
    let list = root_directories.get();
    if !list.is_empty() {
        set_scanning.set(true);
        spawn_local(async move {
            #[derive(Serialize)]
            #[serde(rename_all = "camelCase")]
            struct ScanFilesMultiArgs { root_paths: Vec<String> }
            let args = ScanFilesMultiArgs { root_paths: list.clone() };
            
            let result = match serde_wasm_bindgen::from_value::<Vec<FileListItem>>(
                invoke("scan_files_multi", serde_wasm_bindgen::to_value(&args).unwrap()).await
            ) {
                Ok(files) => {
                    web_sys::console::log_1(&format!("Scan success: {} files", files.len()).into());
                    Some(files)
                },
                Err(e) => {
                    web_sys::console::error_1(&format!("Scan error: {:?}", e).into());
                    None
                }
            };

            set_scanning.set(false);
            if let Some(files) = result {
                set_scanned_files.set(files);
                // Refresh DB files as well
                load_all_files(set_all_files, set_displayed_files, set_file_tags_map).await;
            }
        });
    }
}

pub fn handle_select_directory(
    root_directories: ReadSignal<Vec<String>>,
    set_root_directories: WriteSignal<Vec<String>>,
    set_scanning: WriteSignal<bool>,
    set_scanned_files: WriteSignal<Vec<FileListItem>>,
    set_all_files: WriteSignal<Vec<FileInfo>>,
    set_displayed_files: WriteSignal<Vec<FileInfo>>,
    set_file_tags_map: WriteSignal<std::collections::HashMap<u32, Vec<TagInfo>>>,
    active_root_filter: ReadSignal<Option<String>>,
    set_active_root_filter: WriteSignal<Option<String>>,
) {
    spawn_local(async move {
        let path_val = invoke("select_root_directory", JsValue::NULL).await;
        if let Ok(opt_path) = serde_wasm_bindgen::from_value::<Option<String>>(path_val) {
            if opt_path.is_none() {
                web_sys::console::log_1(&"[Root] selection canceled".into());
                return;
            }
            let path = opt_path.unwrap();
            let mut list = root_directories.get_untracked();
            if !list.iter().any(|p| p == &path) { list.push(path.clone()); }
            set_root_directories.set(list.clone());
            
            // Automatically trigger scan after selecting directory
            set_scanning.set(true);
            #[derive(Serialize)]
            #[serde(rename_all = "camelCase")]
            struct ScanFilesMultiArgs { root_paths: Vec<String> }
            let args = ScanFilesMultiArgs { root_paths: root_directories.get_untracked() };
            
            let scan_result = match serde_wasm_bindgen::from_value::<Vec<FileListItem>>(
                invoke("scan_files_multi", serde_wasm_bindgen::to_value(&args).unwrap()).await
            ) {
                Ok(files) => {
                    web_sys::console::log_1(&format!("Auto-scan success: {} files", files.len()).into());
                    Some(files)
                },
                Err(e) => {
                    web_sys::console::error_1(&format!("Auto-scan error: {:?}", e).into());
                    None
                }
            };

            set_scanning.set(false);
            if let Some(files) = scan_result {
                set_scanned_files.set(files);
                // Refresh DB files as well
                load_all_files(set_all_files, set_displayed_files, set_file_tags_map).await;
            }
            
            web_sys::console::log_1(&"🔍 [FRONTEND] Starting watcher for multiple roots".into());
            spawn_local(async move {
                #[derive(Serialize)]
                #[serde(rename_all = "camelCase")]
                struct StartWatchingMultiArgs { root_paths: Vec<String> }
                let args = StartWatchingMultiArgs { root_paths: root_directories.get_untracked() };
                let _ = invoke("start_watching_multi", serde_wasm_bindgen::to_value(&args).unwrap()).await;
            });
        }
    });
}
