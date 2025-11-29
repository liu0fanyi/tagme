use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

// Lightweight file listing from scan (no hash)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct FileListItem {
    path: String,
    size_bytes: u64,
    last_modified: i64,
    is_directory: bool,
}

// Full file info for files in database (with hash)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct FileInfo {
    id: u32,
    path: String,
    content_hash: String,
    size_bytes: u64,
    last_modified: i64,
    is_directory: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct TagInfo {
    id: u32,
    name: String,
    parent_id: Option<u32>,
    color: Option<String>,
    position: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct FileWithTags {
    file: FileInfo,
    tags: Vec<TagInfo>,
}

#[derive(Clone, Debug, PartialEq, Copy)]
enum SortColumn {
    Name,
    Size,
    Date,
    Type,
}

#[derive(Clone, Debug, PartialEq, Copy)]
enum SortDirection {
    Asc,
    Desc,
}

#[derive(Clone, Debug, PartialEq)]
struct DisplayFile {
    path: String,
    name: String,
    extension: String,
    size_bytes: u64,
    last_modified: i64,
    db_id: Option<u32>,
    tags: Vec<TagInfo>,
    is_directory: bool,
}


#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateTagArgs {
    name: String,
    parent_id: Option<u32>,
    color: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateTagArgs {
    id: u32,
    name: String,
    color: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct DeleteTagArgs {
    id: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MoveTagArgs {
    id: u32,
    new_parent_id: Option<u32>,
    target_position: i32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddFileTagArgs {
    file_path: String,
    tag_id: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoveFileTagArgs {
    file_id: u32,
    tag_id: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetFileTagsArgs {
    file_id: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FilterFilesByTagsArgs {
    tag_ids: Vec<u32>,
    use_and_logic: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScanFilesArgs {
    root_path: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenFileArgs {
    path: String,
}


fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn format_timestamp(ts: i64) -> String {
    // Completely avoid JavaScript Date API - just show raw timestamp or simple format
    if ts <= 0 {
        return "Unknown".to_string();
    }
    
    // Calculate components from Unix timestamp
    // This is 100% Rust, no JavaScript involved
    const SECONDS_PER_MINUTE: i64 = 60;
    const SECONDS_PER_HOUR: i64 = 3600;
    const SECONDS_PER_DAY: i64 = 86400;
    
    let total_days = ts / SECONDS_PER_DAY;
    let remaining_after_days = ts % SECONDS_PER_DAY;
    let hours = remaining_after_days / SECONDS_PER_HOUR;
    let remaining_after_hours = remaining_after_days % SECONDS_PER_HOUR;
    let minutes = remaining_after_hours / SECONDS_PER_MINUTE;
    let seconds = remaining_after_hours % SECONDS_PER_MINUTE;
    
    // Simple readable format without calling any JS Date methods
    format!("{} days, {:02}:{:02}:{:02}", total_days, hours, minutes, seconds)
}

#[component]
pub fn App() -> impl IntoView {
    let (root_directories, set_root_directories) = signal(Vec::<String>::new());
    let (scanned_files, set_scanned_files) = signal(Vec::<FileListItem>::new());
    let (all_files, set_all_files) = signal(Vec::<FileInfo>::new());
    let (all_tags, set_all_tags) = signal(Vec::<TagInfo>::new());
    let (selected_tag_ids, set_selected_tag_ids) = signal(Vec::<u32>::new());
    let (use_and_logic, set_use_and_logic) = signal(true);
    let (displayed_files, set_displayed_files) = signal(Vec::<FileInfo>::new());
    let (file_tags_map, set_file_tags_map) = signal(std::collections::HashMap::<u32, Vec<TagInfo>>::new());
    let (file_tags_map, set_file_tags_map) = signal(std::collections::HashMap::<u32, Vec<TagInfo>>::new());
    let (selected_file_paths, set_selected_file_paths) = signal(Vec::<String>::new());
    let (last_selected_file_path, set_last_selected_file_path) = signal(None::<String>);
    let (file_recommended_tags_map, set_file_recommended_tags_map) = signal(std::collections::HashMap::<u32, Vec<TagInfo>>::new());
    let (file_recommended_info_map, set_file_recommended_info_map) = signal(std::collections::HashMap::<String, Vec<RecommendItem>>::new());
    let (show_recommended, set_show_recommended) = signal(false);
    let (batch_running, set_batch_running) = signal(false);
    let (batch_progress, set_batch_progress) = signal(0usize);
    let (batch_total, set_batch_total) = signal(0usize);
    let (batch_cancel, set_batch_cancel) = signal(false);
    Effect::new(move |_| {
        let running = batch_running.get();
        if let Some(win) = web_sys::window() {
            if let Some(doc) = win.document() {
                if let Some(body) = doc.body() {
                    let _ = body.style().set_property("overflow", if running { "hidden" } else { "" });
                }
            }
        }
        if running {
            web_sys::console::log_1(&"[Overlay] on".into());
            if let Some(win) = web_sys::window() {
                let set_cancel = set_batch_cancel;
                let on_key = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
                    if e.key() == "Escape" { set_cancel.set(true); }
                });
                let _ = win.add_event_listener_with_callback("keydown", on_key.as_ref().unchecked_ref());
                on_key.forget();
            }
        } else {
            web_sys::console::log_1(&"[Overlay] off".into());
        }
    });
    let recommend_all = move |_| {
        if batch_running.get() { return; }
        let files = displayed_files.get();
        let tags = all_tags.get();
        let set_map = set_file_recommended_tags_map;
        let set_info = set_file_recommended_info_map;
        let set_show = set_show_recommended;
        let set_run = set_batch_running;
        let set_prog = set_batch_progress;
        let set_tot = set_batch_total;
        let cancel_sig = batch_cancel;
        spawn_local(async move {
            let total = files.len();
            set_tot.set(total);
            set_prog.set(0);
            set_run.set(true);
            set_show.set(true);
            set_map.set(std::collections::HashMap::new());
            let mut info_map = std::collections::HashMap::new();
            let mut tag_map = std::collections::HashMap::new();
            for (i, f) in files.iter().enumerate() {
                if cancel_sig.get_untracked() { break; }
                let path = f.path.clone();
                let ext = std::path::Path::new(&path).extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()).unwrap_or_default();
                let label_names: Vec<String> = tags.iter().map(|t| t.name.clone()).collect();
                let tk = core::cmp::min(label_names.len(), 8);
                if ["jpg","jpeg","png","webp"].contains(&ext.as_str()) {
                    #[derive(Serialize)]
                    #[serde(rename_all = "camelCase")]
                    struct VisionArgs { image_path: String, labels: Vec<String>, top_k: usize, threshold: f32, base_url: Option<String>, model: Option<String> }
                    let args = VisionArgs { image_path: path.clone(), labels: label_names.clone(), top_k: tk, threshold: 0.6, base_url: Some(String::from("https://api.siliconflow.cn/v1")), model: Some(String::from("deepseek-ai/deepseek-vl2")) };
                    let val = invoke("generate_image_tags_llm", serde_wasm_bindgen::to_value(&args).unwrap()).await;
                    if let Ok(list) = serde_wasm_bindgen::from_value::<Vec<RecommendItem>>(val) { info_map.insert(path.clone(), list.clone()); let mut out: Vec<TagInfo> = Vec::new(); for item in list { if let Some(t) = tags.iter().find(|x| x.name == item.name) { out.push(t.clone()); } } tag_map.insert(f.id, out); }
                } else {
                    #[derive(Serialize)]
                    #[serde(rename_all = "camelCase")]
                    struct LlmArgs { title: String, labels: Vec<String>, top_k: usize, threshold: f32, base_url: Option<String>, model: Option<String> }
                    let title = std::path::Path::new(&path).file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
                    if !title.is_empty() {
                        let args = LlmArgs { title: title.clone(), labels: label_names.clone(), top_k: tk, threshold: 0.6, base_url: Some(String::from("https://api.siliconflow.cn/v1")), model: Some(String::from("deepseek-ai/DeepSeek-V3.2-Exp")) };
                        let val = invoke("generate_tags_llm", serde_wasm_bindgen::to_value(&args).unwrap()).await;
                        if let Ok(list) = serde_wasm_bindgen::from_value::<Vec<RecommendItem>>(val) { info_map.insert(path.clone(), list.clone()); let mut out: Vec<TagInfo> = Vec::new(); for item in list { if let Some(t) = tags.iter().find(|x| x.name == item.name) { out.push(t.clone()); } } tag_map.insert(f.id, out); }
                    }
                }
                set_prog.set(i + 1);
                if i % 5 == 4 { set_map.set(tag_map.clone()); set_info.set(info_map.clone()); }
            }
            set_map.set(tag_map);
            set_info.set(info_map);
            set_run.set(false);
            set_batch_cancel.set(false);
        });
    };
    let (scanning, set_scanning) = signal(false);
    let (show_add_tag_dialog, set_show_add_tag_dialog) = signal(false);
    let (new_tag_name, set_new_tag_name) = signal(String::new());
    let (new_tag_parent, set_new_tag_parent) = signal(None::<u32>);
    let (new_tag_input_sidebar, set_new_tag_input_sidebar) = signal(String::new());
    let (show_purge_confirm, set_show_purge_confirm) = signal(false);
    let (show_delete_tag_confirm, set_show_delete_tag_confirm) = signal(false);
    let (delete_target_tag_id, set_delete_target_tag_id) = signal(None::<u32>);
    let (show_update_modal, set_show_update_modal) = signal(false);
    let (update_current, set_update_current) = signal(String::new());
    let (update_latest, set_update_latest) = signal(String::new());
    let (update_has, set_update_has) = signal(false);
    
    // Sorting state
    let (sort_column, set_sort_column) = signal(SortColumn::Name);
    let (sort_direction, set_sort_direction) = signal(SortDirection::Asc);
    let (active_root_filter, set_active_root_filter) = signal(None::<String>);

    // Panel resizing state
    let (left_panel_width, set_left_panel_width) = signal(300.0);
    let (right_panel_width, set_right_panel_width) = signal(300.0);
    let (is_resizing_left, set_is_resizing_left) = signal(false);
    let (is_resizing_right, set_is_resizing_right) = signal(false);

    // Derived signal for sorted files
    let sorted_files = move || {
        let scanned = scanned_files.get();
        let db = displayed_files.get();
        let tags_map = file_tags_map.get();
        
        let mut display_files: Vec<DisplayFile> = Vec::new();
        let mut seen_paths = std::collections::HashSet::new();

        // Add DB files first
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

        // Add scanned files that are not in DB (only when no tag filter is active)
        let has_tag_filter = !selected_tag_ids.get().is_empty();
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

        // Sort
        let col = sort_column.get();
        let dir = sort_direction.get();
        
        display_files.sort_by(|a, b| {
            let cmp = match col {
                SortColumn::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortColumn::Size => a.size_bytes.cmp(&b.size_bytes),
                SortColumn::Date => a.last_modified.cmp(&b.last_modified),
                SortColumn::Type => a.extension.to_lowercase().cmp(&b.extension.to_lowercase()),
            };
            
            match dir {
                SortDirection::Asc => cmp,
                SortDirection::Desc => cmp.reverse(),
            }
        });

        display_files
    };

    let toggle_sort = move |col: SortColumn| {
        if sort_column.get() == col {
            set_sort_direction.update(|d| *d = match d {
                SortDirection::Asc => SortDirection::Desc,
                SortDirection::Desc => SortDirection::Asc,
            });
        } else {
            set_sort_column.set(col);
            set_sort_direction.set(SortDirection::Asc);
        }
    };
    
    // Drag and drop state
    let (dragging_tag_id, set_dragging_tag_id) = signal(None::<u32>);
    let (drop_target_tag_id, set_drop_target_tag_id) = signal(None::<u32>);
    let (drop_position, set_drop_position) = signal(0.5f64); // 0.0=top, 1.0=bottom
    let (reload_tags_trigger, set_reload_tags_trigger) = signal(0u32);
    let (last_click_time, set_last_click_time) = signal(0.0);
    let (is_maximized, set_is_maximized) = signal(false);

    // Global mouse up handler for drag and drop
    Effect::new(move |_| {
        let window = web_sys::window().unwrap();
        
        let on_mouseup = Closure::<dyn FnMut(_)>::new(move |_ev: web_sys::MouseEvent| {
            if let Some(dragged_id) = dragging_tag_id.get_untracked() {
                web_sys::console::log_1(&format!("🔵 Mouse up - dragged_id: {}", dragged_id).into());
                
                if let Some(target_id) = drop_target_tag_id.get_untracked() {
                    web_sys::console::log_1(&format!("🔵 Drop target: {}", target_id).into());
                    
                    let pos = drop_position.get_untracked();
                    web_sys::console::log_1(&format!("📍 Drop position: {:.2}", pos).into());
                    
                    if dragged_id != target_id {
                        // Check for cycles
                        let tags = all_tags.get_untracked();
                        let mut is_descendant = false;
                        let mut check_id = Some(target_id);
                        while let Some(curr) = check_id {
                            if curr == dragged_id {
                                is_descendant = true;
                                break;
                            }
                            check_id = tags.iter().find(|t| t.id == curr).and_then(|t| t.parent_id);
                        }

                        if !is_descendant {
                            web_sys::console::log_1(&format!("✅ Valid drop - moving tag {} under {}", dragged_id, target_id).into());

                            // Calculate target position and parent based on drop position
                            let tags = all_tags.get_untracked();
                            let target_tag = tags.iter().find(|t| t.id == target_id);

                            let (new_parent_id, target_position, action) = if let Some(tag) = target_tag {
                                if pos < 0.25 {
                                    // Insert before target tag (same parent)
                                    if tag.parent_id == tags.iter().find(|t| t.id == dragged_id).and_then(|t| t.parent_id) {
                                        // Moving within same parent - need special handling
                                        let current_pos = tags.iter().find(|t| t.id == dragged_id).map(|t| t.position).unwrap_or(0);
                                        if current_pos < tag.position {
                                            // Moving forward, use target's position
                                            (tag.parent_id, tag.position, "before-same-parent")
                                        } else {
                                            // Moving backward, use target's position
                                            (tag.parent_id, tag.position, "before-same-parent")
                                        }
                                    } else {
                                        // Moving to different parent
                                        (tag.parent_id, tag.position, "before")
                                    }
                                } else if pos > 0.75 {
                                    // Insert after target tag (same parent)
                                    (tag.parent_id, tag.position + 1, "after")
                                } else {
                                    // As child of target tag
                                    (Some(target_id), 0, "child")
                                }
                            } else {
                                (None, 0, "root")
                            };

                            web_sys::console::log_1(&format!("🎯 Action: {}, Parent: {:?}, Position: {}", action, new_parent_id, target_position).into());

                            spawn_local(async move {
                                let args = MoveTagArgs {
                                    id: dragged_id,
                                    new_parent_id,
                                    target_position,
                                };
                                let _ = invoke("move_tag", serde_wasm_bindgen::to_value(&args).unwrap()).await;
                                // Trigger reload
                                set_reload_tags_trigger.update(|v| *v += 1);
                            });
                        } else {
                            web_sys::console::log_1(&"⚠️ Cannot drop - would create cycle".into());
                        }
                    }
                }
                
                set_dragging_tag_id.set(None);
                set_drop_target_tag_id.set(None);
            }
        });
        
        let _ = window.add_event_listener_with_callback("mouseup", on_mouseup.as_ref().unchecked_ref());
        on_mouseup.forget();
    });

    // Global mouse handlers for panel resizing
    Effect::new(move |_| {
        let window = web_sys::window().unwrap();
        
        // Mouse move handler for resizing
        let on_mousemove = Closure::<dyn FnMut(_)>::new(move |ev: web_sys::MouseEvent| {
            if is_resizing_left.get_untracked() {
                let x = ev.client_x() as f64;
                let new_width = x.max(200.0).min(600.0); // Min 200px, max 600px
                web_sys::console::log_1(&format!("Resizing left panel to: {}", new_width).into());
                set_left_panel_width.set(new_width);
            } else if is_resizing_right.get_untracked() {
                let window_width = web_sys::window().unwrap().inner_width().unwrap().as_f64().unwrap();
                let x = ev.client_x() as f64;
                let new_width = (window_width - x).max(200.0).min(600.0);
                web_sys::console::log_1(&format!("Resizing right panel to: {}", new_width).into());
                set_right_panel_width.set(new_width);
            }
        });
        
        let _ = window.add_event_listener_with_callback("mousemove", on_mousemove.as_ref().unchecked_ref());
        on_mousemove.forget();
        
        // Mouse up handler to stop resizing
        let on_mouseup_resize = Closure::<dyn FnMut(_)>::new(move |_ev: web_sys::MouseEvent| {
            web_sys::console::log_1(&"Mouse up - stopping resize".into());
            set_is_resizing_left.set(false);
            set_is_resizing_right.set(false);
        });
        
        let _ = window.add_event_listener_with_callback("mouseup", on_mouseup_resize.as_ref().unchecked_ref());
        on_mouseup_resize.forget();
    });

    // Effect to reload tags when trigger changes
    Effect::new(move |_| {
        reload_tags_trigger.get(); // Track the trigger
        if reload_tags_trigger.get_untracked() > 0 {
            spawn_local(async move {
                load_tags(set_all_tags).await;
            });
        }
    });

    // Load initial state
    Effect::new(move || {
        spawn_local(async move {
            let roots: Result<Vec<String>, _> = serde_wasm_bindgen::from_value(
                invoke("get_root_directories", JsValue::NULL).await
            );
            match roots {
                Ok(list) => {
                    if list.is_empty() {
                        let root: Option<String> = serde_wasm_bindgen::from_value(
                            invoke("get_root_directory", JsValue::NULL).await
                        ).unwrap_or(None);
                        if let Some(p) = root { set_root_directories.set(vec![p]); }
                    } else {
                        set_root_directories.set(list);
                    }
                },
                Err(_) => {
                    let root: Option<String> = serde_wasm_bindgen::from_value(
                        invoke("get_root_directory", JsValue::NULL).await
                    ).unwrap_or(None);
                    if let Some(p) = root { set_root_directories.set(vec![p]); }
                }
            }

            // Load tags
            load_tags(set_all_tags).await;

            // Load all files
            load_all_files(set_all_files, set_displayed_files, set_file_tags_map).await;

            // Load window state
            let state_value = invoke("load_window_state", JsValue::NULL).await;
            let _ = state_value; // Unused for now
            
            let list = root_directories.get_untracked();
            if !list.is_empty() {
                spawn_local(async move {
                    #[derive(Serialize)]
                    #[serde(rename_all = "camelCase")]
                    struct StartWatchingMultiArgs { root_paths: Vec<String> }
                    let args = StartWatchingMultiArgs { root_paths: list.clone() };
                    let _ = invoke("start_watching_multi", serde_wasm_bindgen::to_value(&args).unwrap()).await;
                });
            }

            let list2 = root_directories.get_untracked();
            if !list2.is_empty() {
                spawn_local(async move {
                    #[derive(Serialize)]
                    #[serde(rename_all = "camelCase")]
                    struct ScanFilesMultiArgs { root_paths: Vec<String> }
                    let args = ScanFilesMultiArgs { root_paths: list2.clone() };
                    if let Ok(files) = serde_wasm_bindgen::from_value::<Vec<FileListItem>>(
                        invoke("scan_files_multi", serde_wasm_bindgen::to_value(&args).unwrap()).await
                    ) {
                        set_scanned_files.set(files);
                        load_all_files(set_all_files, set_displayed_files, set_file_tags_map).await;
                    }
                });
            }
            
            // Setup file system change listener
            let setup_listener = js_sys::Function::new_no_args(r#"
                console.log('🔧 [FRONTEND] Setting up Tauri event listener...');
                if (window.__TAURI__ && window.__TAURI__.event) {
                    window.__TAURI__.event.listen('file-system-change', () => {
                        console.log('📬 [FRONTEND] File change detected by Tauri');
                        window.dispatchEvent(new CustomEvent('tauri-fs-change'));
                        console.log('✅ [FRONTEND] Custom event dispatched');
                    });
                    console.log('✅ [FRONTEND] Tauri event listener registered');
                } else {
                    console.error('❌ [FRONTEND] Tauri event API not available');
                }
            "#);
            let _ = setup_listener.call0(&JsValue::NULL);
        });
    });
    
    // Listen for custom file change events and trigger scan
    Effect::new(move |_| {
        let window = web_sys::window().expect("no window");
        web_sys::console::log_1(&"🎧 [FRONTEND] Registering custom event listener for 'tauri-fs-change'".into());
        let closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
            web_sys::console::log_1(&"📥 [FRONTEND] Custom event received, refreshing file list...".into());
            let list = root_directories.get_untracked();
            if !list.is_empty() {
                set_scanning.set(true);
                spawn_local(async move {
                    #[derive(Serialize)]
                    #[serde(rename_all = "camelCase")]
                    struct ScanFilesMultiArgs { root_paths: Vec<String> }
                    let args = ScanFilesMultiArgs { root_paths: list.clone() };
                    if let Ok(files) = serde_wasm_bindgen::from_value::<Vec<FileListItem>>(
                        invoke("scan_files_multi", serde_wasm_bindgen::to_value(&args).unwrap()).await
                    ) {
                        set_scanned_files.set(files);
                        load_all_files(set_all_files, set_displayed_files, set_file_tags_map).await;
                    }
                    set_scanning.set(false);
                });
            }
        }) as Box<dyn FnMut(_)>);
        
        let _ = window.add_event_listener_with_callback("tauri-fs-change", closure.as_ref().unchecked_ref());
        web_sys::console::log_1(&"✅ [FRONTEND] Custom event listener registered".into());
        closure.forget();
    });

    let select_directory = move |_| {
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
                
                // Tauri unwraps Result automatically, so expect Vec<FileListItem> directly
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
    };

    let scan_directory = move |_| {
        let list = root_directories.get();
        if !list.is_empty() {
            set_scanning.set(true);
            spawn_local(async move {
                #[derive(Serialize)]
                #[serde(rename_all = "camelCase")]
                struct ScanFilesMultiArgs { root_paths: Vec<String> }
                let args = ScanFilesMultiArgs { root_paths: list.clone() };
                
                // Tauri unwraps Result automatically, so expect Vec<FileListItem> directly
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
    };

    
    let close = move |_| {
        spawn_local(async move {
            let _ = invoke("close_window", JsValue::NULL).await;
        });
    };

    let minimize = move |_| {
        spawn_local(async move {
            let _ = invoke("minimize_window", JsValue::NULL).await;
        });
    };

    let toggle_maximize = move |_| {
        set_is_maximized.update(|v| *v = !*v);
        spawn_local(async move {
            let _ = invoke("toggle_maximize", JsValue::NULL).await;
        });
    };



    let toggle_tag_selection = move |tag_id: u32| {
        let mut current = selected_tag_ids.get();
        web_sys::console::log_1(&format!("toggle_tag_selection start, tag_id={}, before={:?}", tag_id, current).into());
        let tags = all_tags.get();
        let mut stack = vec![tag_id];
        let mut subtree_ids: Vec<u32> = Vec::new();
        while let Some(id) = stack.pop() {
            subtree_ids.push(id);
            for t in tags.iter().filter(|t| t.parent_id == Some(id)) {
                stack.push(t.id);
            }
        }
        let should_select = !current.iter().any(|&id| id == tag_id);
        web_sys::console::log_1(&format!("should_select={}, subtree_ids={:?}", should_select, subtree_ids).into());
        if should_select {
            for id in &subtree_ids {
                if !current.contains(id) {
                    current.push(*id);
                }
            }
        } else {
            let remove_set: std::collections::HashSet<u32> = subtree_ids.iter().copied().collect();
            current.retain(|id| !remove_set.contains(id));
        }
        web_sys::console::log_1(&format!("toggle_tag_selection end, after={:?}", current).into());
        set_selected_tag_ids.set(current.clone());
        let force_or = should_select && subtree_ids.len() > 1;
        let logic = if force_or {
            set_use_and_logic.set(false);
            false
        } else {
            use_and_logic.get()
        };
        web_sys::console::log_1(&format!("filter_files with {} tags, use_and={}, force_or={}", current.len(), logic, force_or).into());
        filter_files(current, logic, set_displayed_files, all_files.get());
    };

    let toggle_and_or = move |_| {
        let new_logic = !use_and_logic.get();
        set_use_and_logic.set(new_logic);
        filter_files(selected_tag_ids.get(), new_logic, set_displayed_files, all_files.get());
    };

    let show_all = move |_| {
        set_selected_tag_ids.set(Vec::new());
        set_displayed_files.set(all_files.get());
    };

    let toggle_file_selection = move |file_path: String| {
        let mut current = selected_file_paths.get();
        if let Some(pos) = current.iter().position(|p| p == &file_path) {
            current.remove(pos);
        } else {
            current.push(file_path);
        }
        set_selected_file_paths.set(current);
    };

    let _add_tag_to_selected_files = move |tag_id: u32| {
        let file_paths = selected_file_paths.get();
        for file_path in file_paths {
            spawn_local(async move {
                let args = AddFileTagArgs { file_path, tag_id };
                let _ = invoke("add_file_tag", serde_wasm_bindgen::to_value(&args).unwrap()).await;
            });
        }
        // Reload file tags after adding
        spawn_local(async move {
            load_all_files(set_all_files, set_displayed_files, set_file_tags_map).await;
        });
    };

    let create_tag_action = move |_| {
        let name = new_tag_name.get();
        let parent = new_tag_parent.get();
        if !name.is_empty() {
            spawn_local(async move {
                let args = CreateTagArgs { name, parent_id: parent, color: None };
                let _ = invoke("create_tag", serde_wasm_bindgen::to_value(&args).unwrap()).await;
                load_tags(set_all_tags).await;
                set_show_add_tag_dialog.set(false);
                set_new_tag_name.set(String::new());
                set_new_tag_parent.set(None);
            });
        }
    };

    view! {
        <div class="app">
            <div class="header"
                on:mousedown=move |e| {
                    let now = js_sys::Date::now();
                    let last = last_click_time.get_untracked();
                    set_last_click_time.set(now);

                    let target = e.target();
                    if let Some(element) = target.and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok()) {
                        // Check if clicking on a button or inside a button
                        if element.closest("button").ok().flatten().is_none() {
                            if now - last < 300.0 {
                                // Double click detected
                                toggle_maximize(());
                            } else {
                                // Single click - start drag
                                spawn_local(async move {
                                    let _ = invoke("start_drag", JsValue::NULL).await;
                                });
                            }
                        }
                    }
                }
            >
                <h1>"TagMe"</h1>
                <div class="header-buttons">
                    <button on:click=move |_| minimize(()) class="header-btn" title="Minimize">
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" style="pointer-events: none;">
                            <path d="M19 13H5v-2h14v2z"/>
                        </svg>
                    </button>
                    <button on:click=move |_| toggle_maximize(()) class="header-btn" title=move || if is_maximized.get() { "Restore" } else { "Maximize" }>
                        {move || if is_maximized.get() {
                            view! {
                                <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" style="pointer-events: none;">
                                    <path d="M4 8h4V4h12v12h-4v4H4V8zm2 2v8h8v-2h4V6H10v2H6z"/>
                                </svg>
                            }
                        } else {
                            view! {
                                <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" style="pointer-events: none;">
                                    <path d="M4 4h16v16H4V4zm2 2v12h12V6H6z"/>
                                </svg>
                            }
                        }}
                    </button>
                    <button on:click=move |_| close(()) class="header-btn" title="Close">
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" style="pointer-events: none;">
                            <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/>
                        </svg>
                    </button>
                </div>
            </div>

            <div class="toolbar">
                <button on:click=select_directory>"Select Root Directory"</button>
                {move || {
                    let list = root_directories.get();
                    if list.is_empty() { None } else {
                        Some(view! {
                            <div class="root-paths" style="display:flex; gap:6px; align-items:center;">
                                <For
                                    each=move || list.clone()
                                    key=|p| p.clone()
                                    children=move |p| {
                                        let rp = p.clone();
                                        let rp_display = rp.clone();
                                        let remove_val = rp.clone();
                                        let remove = move |ev: web_sys::MouseEvent| {
                                            ev.stop_propagation();
                                            let rp2 = remove_val.clone();
                                            spawn_local(async move {
                                                #[derive(Serialize)]
                                                #[serde(rename_all = "camelCase")]
                                                struct RemoveRootArgs { path: String }
                                                let _ = invoke("remove_root_directory", serde_wasm_bindgen::to_value(&RemoveRootArgs { path: rp2.clone() }).unwrap()).await;
                                                let do_purge = web_sys::window().and_then(|w| w.confirm_with_message(&format!("Also purge DB records under root?\n{}", rp2)).ok()).unwrap_or(false);
                                                if do_purge {
                                                    #[derive(Serialize)]
                                                    #[serde(rename_all = "camelCase")]
                                                    struct PurgeArgs { path: String }
                                                    let res = invoke("purge_files_under_root", serde_wasm_bindgen::to_value(&PurgeArgs { path: rp2.clone() }).unwrap()).await;
                                                    if let Ok(cnt) = serde_wasm_bindgen::from_value::<u32>(res) {
                                                        web_sys::console::log_1(&format!("[DB] purged {} files under root", cnt).into());
                                                    }
                                                }
                                                // Reload roots from backend to ensure persistence, then restart watcher and refresh files
                                                let roots_val = invoke("get_root_directories", JsValue::NULL).await;
                                                if let Ok(roots) = serde_wasm_bindgen::from_value::<Vec<String>>(roots_val) {
                                                    set_root_directories.set(roots.clone());
                                                    // Clear active filter if it pointed to removed path
                                                    if active_root_filter.get_untracked().as_ref() == Some(&rp2) {
                                                        set_active_root_filter.set(None);
                                                    }
                                                    // Restart watcher
                                                    #[derive(Serialize)]
                                                    #[serde(rename_all = "camelCase")]
                                                    struct StartWatchingMultiArgs { root_paths: Vec<String> }
                                                    let _ = invoke("start_watching_multi", serde_wasm_bindgen::to_value(&StartWatchingMultiArgs { root_paths: roots.clone() }).unwrap()).await;
                                                    // Refresh DB files and displayed files
                                                    load_all_files(set_all_files, set_displayed_files, set_file_tags_map).await;
                                                }
                                            });
                                            set_root_directories.update(|v| v.retain(|x| x != &remove_val));
                                            let updated = root_directories.get_untracked();
                                            spawn_local(async move {
                                                #[derive(Serialize)]
                                                #[serde(rename_all = "camelCase")]
                                                struct StartWatchingMultiArgs { root_paths: Vec<String> }
                                                let _ = invoke("start_watching_multi", serde_wasm_bindgen::to_value(&StartWatchingMultiArgs { root_paths: updated.clone() }).unwrap()).await;
                                            });
                                        };
                                        let rp_filter_src = rp.clone();
                                        let rp_filter = rp_filter_src.clone();
                                        let is_active = move || active_root_filter.get().as_ref().map(|x| x == &rp_filter).unwrap_or(false);
                                        let toggle_val = rp_filter_src.clone();
                                        let toggle_filter = move |_| {
                                            let current = active_root_filter.get_untracked();
                                            if current.as_ref() == Some(&toggle_val) {
                                                set_active_root_filter.set(None);
                                            } else {
                                                set_active_root_filter.set(Some(toggle_val.clone()));
                                            }
                                        };
                                        view! {
                                            <span
                                                class=move || if is_active() { "root-path active" } else { "root-path" }
                                                style="padding:2px 6px; border:1px solid #ccc; border-radius:4px; display:inline-flex; align-items:center; gap:6px; cursor:pointer;"
                                                on:click=toggle_filter
                                            >
                                                {rp_display.clone()}
                                                <button on:click=remove title="Remove" style="border:none; background:transparent; cursor:pointer; color:#c00;">"×"</button>
                                            </span>
                                        }
                                    }
                                />
                            </div>
                        })
                    }
                }}
                <button on:click=scan_directory disabled=move || root_directories.get().is_empty()>
                    {move || if scanning.get() { "Scanning..." } else { "Scan Files" }}
                </button>
                <button on:click={
                    let set_modal = set_show_update_modal;
                    let set_c = set_update_current;
                    let set_l = set_update_latest;
                    let set_h = set_update_has;
                    move |_| {
                        spawn_local(async move {
                            let val = invoke("updater_check", JsValue::NULL).await;
                            match serde_wasm_bindgen::from_value::<UpdateInfo>(val.clone()) {
                                Ok(info) => { set_c.set(info.current); set_l.set(info.latest.unwrap_or_default()); set_h.set(info.has_update); set_modal.set(true); },
                                Err(e) => { web_sys::console::error_1(&format!("[UI] updater_check error: {:?}; raw={:?}", e, val).into()); }
                            }
                        });
                    }
                }>
                    "Check Updates"
                </button>
                <button on:mousedown={move |_| {
                        web_sys::console::log_1(&"[UI] Clear DB Files mousedown".into());
                    }}
                    on:click={move |_| {
                        set_show_purge_confirm.set(true);
                    }}
                >
                    "Clear DB Files"
                </button>
            </div>

            <div class="main-content">
                <div class="left-panel" style=move || format!("width: {}px", left_panel_width.get())>
                    <div class="panel-header">
                        <h2>"Tags"</h2>
                        <button on:click=move |_| set_show_add_tag_dialog.set(true)>"+"</button>
                    </div>
                    <TagTree
                        tags=all_tags
                        selected_tag_ids=selected_tag_ids
                        set_selected_tag_ids=set_selected_tag_ids
                        use_and_logic=use_and_logic
                        set_displayed_files=set_displayed_files
                        all_files=all_files
                        on_toggle=toggle_tag_selection
                        _set_all_tags=set_all_tags
                        dragging_tag_id=dragging_tag_id
                        set_dragging_tag_id=set_dragging_tag_id
                        drop_target_tag_id=drop_target_tag_id
                        set_drop_target_tag_id=set_drop_target_tag_id
                        drop_position=drop_position
                        set_drop_position=set_drop_position
                        set_reload_tags_trigger=set_reload_tags_trigger
                        set_show_delete_tag_confirm=set_show_delete_tag_confirm
                        set_delete_target_tag_id=set_delete_target_tag_id
                    />
                </div>

                <div
                    class="resizer"
                    on:mousedown=move |_| {
                        web_sys::console::log_1(&"Left resizer mousedown".into());
                        set_is_resizing_left.set(true);
                    }
                ></div>

                <div class="center-panel">
                    <div class="panel-header">
                        <h2>"Files"</h2>
                        <div class="file-controls">
                            <button on:click=show_all>"Show All"</button>
                            <button on:click=toggle_and_or>
                                {move || if use_and_logic.get() { "Filter: AND" } else { "Filter: OR" }}
                            </button>
                            <button on:click=recommend_all>"Recommend All"</button>
                            <button on:click=move |_| {
                                set_show_recommended.set(false);
                                set_file_recommended_tags_map.set(std::collections::HashMap::new());
                                web_sys::console::log_1(&"[Recommend] cleared".into());
                            }>
                                "Hide AI"
                            </button>
                        
                        </div>
                    </div>
                    <GroupedFileList
                        files=sorted_files
                        roots=root_directories
                        active_root_filter=active_root_filter
                        selected_file_paths=selected_file_paths
                        on_toggle=toggle_file_selection
                        sort_column=sort_column
                        sort_direction=sort_direction
                        on_sort=toggle_sort
                        set_selected_file_paths=set_selected_file_paths
                        last_selected_file_path=last_selected_file_path
                        set_last_selected_file_path=set_last_selected_file_path
                        _recommended_map=file_recommended_tags_map
                        recommended_info_map=file_recommended_info_map
                        show_recommended=show_recommended
                        all_tags=all_tags
                        set_all_files=set_all_files
                        set_displayed_files=set_displayed_files
                        set_file_tags_map=set_file_tags_map
                    />
                </div>

                <div
                    class="resizer"
                    on:mousedown=move |_| {
                        web_sys::console::log_1(&"Right resizer mousedown".into());
                        set_is_resizing_right.set(true);
                    }
                ></div>

                        <div class="right-sidebar" style=move || format!("width: {}px", right_panel_width.get())>
                    <div class="panel-header">
                        <h2>"File Tags"</h2>
                        <div class="file-controls">
                            <button on:click={
                                let tags_sig = all_tags.clone();
                                let sel = selected_file_paths.clone();
                                let set_info = set_file_recommended_info_map;
                                let set_show = set_show_recommended;
                                let set_run = set_batch_running;
                                let set_prog = set_batch_progress;
                                let set_tot = set_batch_total;
                                let cancel_sig = batch_cancel;
                                move |_| {
                                    let files = sel.get();
                                    if files.is_empty() { return; }
                                    let tags = tags_sig.get();
                                    let label_names: Vec<String> = tags.iter().map(|t| t.name.clone()).collect();
                                    let tk = core::cmp::min(label_names.len(), 8);
                                    set_tot.set(files.len());
                                    set_prog.set(0);
                                    set_run.set(true);
                                    set_show.set(true);
                                    spawn_local(async move {
                                        let mut done = 0usize;
                                        for path in files {
                                            if cancel_sig.get_untracked() { break; }
                                            let ext = std::path::Path::new(&path).extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()).unwrap_or_default();
                                            if ["jpg","jpeg","png","webp"].contains(&ext.as_str()) {
                                                #[derive(Serialize)]
                                                #[serde(rename_all = "camelCase")]
                                                struct VisionArgs { image_path: String, labels: Vec<String>, top_k: usize, threshold: f32, base_url: Option<String>, model: Option<String> }
                                                let args = VisionArgs { image_path: path.clone(), labels: label_names.clone(), top_k: tk, threshold: 0.6, base_url: Some(String::from("https://api.siliconflow.cn/v1")), model: Some(String::from("deepseek-ai/deepseek-vl2")) };
                                                let val = invoke("generate_image_tags_llm", serde_wasm_bindgen::to_value(&args).unwrap()).await;
                                                if let Ok(list) = serde_wasm_bindgen::from_value::<Vec<RecommendItem>>(val) {
                                                    web_sys::console::log_1(&format!("[VL] items=[{}]", list.iter().map(|ri| format!("{}:{:.3}:{}", ri.name, ri.score, ri.source)).collect::<Vec<_>>().join(", ")).into());
                                                    let mut map = file_recommended_info_map.get_untracked();
                                                    map.insert(path.clone(), list);
                                                    set_info.set(map);
                                                }
                                            } else {
                                                #[derive(Serialize)]
                                                #[serde(rename_all = "camelCase")]
                                                struct LlmArgs { title: String, labels: Vec<String>, top_k: usize, threshold: f32, base_url: Option<String>, model: Option<String> }
                                                let title = std::path::Path::new(&path).file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
                                                if !title.is_empty() {
                                                    let args = LlmArgs { title: title.clone(), labels: label_names.clone(), top_k: tk, threshold: 0.6, base_url: Some(String::from("https://api.siliconflow.cn/v1")), model: Some(String::from("deepseek-ai/DeepSeek-V3.2-Exp")) };
                                                    let val = invoke("generate_tags_llm", serde_wasm_bindgen::to_value(&args).unwrap()).await;
                                                    if let Ok(list) = serde_wasm_bindgen::from_value::<Vec<RecommendItem>>(val) {
                                                        web_sys::console::log_1(&format!("[LLM] items=[{}]", list.iter().map(|ri| format!("{}:{:.3}:{}", ri.name, ri.score, ri.source)).collect::<Vec<_>>().join(", ")).into());
                                                        let mut map = file_recommended_info_map.get_untracked();
                                                        map.insert(path.clone(), list);
                                                        set_info.set(map);
                                                    }
                                                }
                                            }
                                            done += 1;
                                            set_prog.set(done);
                                        }
                                        set_run.set(false);
                                        set_batch_cancel.set(false);
                                    });
                                }
                            }>"Recommend Tag"</button>
                        </div>
                    </div>
                    {move || {
                        let files = selected_file_paths.get();
                        let is_empty = files.is_empty();
                        let count = files.len();
                        
                        let header = if is_empty {
                            "No files selected".to_string()
                        } else if count == 1 {
                            files[0].split("\\\\").last().unwrap_or(&files[0]).to_string()
                        } else {
                            format!("{} files selected", count)
                        };

                        view! {
                            <div class="tag-panel">
                                <h3>{header}</h3>
                                <Show when=move || !is_empty>
                                    <div class="new-tag-input">
                                        <input
                                            type="text"
                                            placeholder="Type tag name and press Enter..."
                                            prop:value=new_tag_input_sidebar
                                            on:input=move |e| set_new_tag_input_sidebar.set(event_target_value(&e))
                                            on:keydown=move |e| {
                                                if e.key() == "Enter" {
                                                    let name = new_tag_input_sidebar.get().trim().to_string();
                                                    if !name.is_empty() {
                                                        let paths = selected_file_paths.get();
                                                        spawn_local(async move {
                                                            let args = CreateTagArgs { name: name.clone(), parent_id: None, color: None };
                                                            let result = invoke("create_tag", serde_wasm_bindgen::to_value(&args).unwrap()).await;
                                                            
                                                            if let Ok(tid) = serde_wasm_bindgen::from_value::<u32>(result) {
                                                                for p in &paths {
                                                                    let pc = p.clone();
                                                                    let args2 = AddFileTagArgs { file_path: pc, tag_id: tid };
                                                                    let _ = invoke("add_file_tag", serde_wasm_bindgen::to_value(&args2).unwrap()).await;
                                                                }
                                                                load_tags(set_all_tags).await;
                                                                load_all_files(set_all_files, set_displayed_files, set_file_tags_map).await;
                                                            }
                                                        });
                                                        set_new_tag_input_sidebar.set(String::new());
                                                    }
                                                }
                                            }
                                        />
                                    </div>
                                    <div class="tag-list">
                                        <For
                                            each=move || all_tags.get()
                                            key=|t| t.id
                                            children=move |t| {
                                                let tid = t.id;
                                                let tname = t.name.clone();
                                                
                                                // Check if all selected files have this tag
                                                let is_checked = move || {
                                                    let files = selected_file_paths.get();
                                                    if files.is_empty() {
                                                        return false;
                                                    }
                                                    
                                                    let tags_map = file_tags_map.get();
                                                    let all_files_info = all_files.get();
                                                    
                                                    // Check if all selected files have this tag
                                                    files.iter().all(|file_path| {
                                                        // Find file by path
                                                        if let Some(file_info) = all_files_info.iter().find(|f| &f.path == file_path) {
                                                            // Check if file has this tag
                                                            if let Some(file_tags) = tags_map.get(&file_info.id) {
                                                                file_tags.iter().any(|tag| tag.id == tid)
                                                            } else {
                                                                false
                                                            }
                                                        } else {
                                                            false
                                                        }
                                                    })
                                                };
                                                
                                                view! {
                                                    <label class="tag-item">
                                                        <input
                                                            type="checkbox"
                                                            checked=is_checked
                                                            on:change=move |e| {
                                                                let checked = event_target_checked(&e);
                                                                let ps = selected_file_paths.get();
                                                                
                                                                if checked {
                                                                    // Add tag to all selected file paths (DB entry will be created if missing)
                                                                    for p in &ps {
                                                                        let pc = p.clone();
                                                                        spawn_local(async move {
                                                                            let args = AddFileTagArgs { file_path: pc, tag_id: tid };
                                                                            let _ = invoke("add_file_tag", serde_wasm_bindgen::to_value(&args).unwrap()).await;
                                                                        });
                                                                    }
                                                                } else {
                                                                    // Remove tag only from files present in DB
                                                                    let all_files_info = all_files.get();
                                                                    for p in &ps {
                                                                        if let Some(file_info) = all_files_info.iter().find(|f| &f.path == p) {
                                                                            let file_id = file_info.id;
                                                                            spawn_local(async move {
                                                                                let args = RemoveFileTagArgs { file_id, tag_id: tid };
                                                                                let _ = invoke("remove_file_tag", serde_wasm_bindgen::to_value(&args).unwrap()).await;
                                                                            });
                                                                        }
                                                                    }
                                                                }
                                                                
                                                                // Reload only the affected files immediately
                                                                spawn_local(async move {
                                                                    load_all_files(set_all_files, set_displayed_files, set_file_tags_map).await;
                                                                });
                                                            }
                                                        />
                                                        <span style=t.color.map(|c| format!("color: {}", c)).unwrap_or_default()>{tname}</span>
                                                    </label>
                                                }
                                            }
                                        />
                                    </div>
                                </Show>
                            </div>
                        }
                    }}
                </div>
            </div>

            {move || show_add_tag_dialog.get().then(|| view! {
                <div class="modal-overlay" on:click=move |_| set_show_add_tag_dialog.set(false)>
                    <div class="modal" on:click={|e| e.stop_propagation()}>
                        <h3>"Add New Tag"</h3>
                        <input
                            type="text"
                            placeholder="Tag name"
                            prop:value=new_tag_name
                            on:input=move |e| set_new_tag_name.set(event_target_value(&e))
                        />
                        <button on:click=create_tag_action>"Create"</button>
                        <button on:click=move |_| set_show_add_tag_dialog.set(false)>"Cancel"</button>
                    </div>
                </div>
            })}

            {move || show_purge_confirm.get().then(|| view! {
                <div class="modal-overlay" on:click=move |_| set_show_purge_confirm.set(false)>
                    <div class="modal" on:click={|e| e.stop_propagation()}>
                        <h3>"Confirm Purge"</h3>
                        <p>"Purge ALL files in database? This cannot be undone."</p>
                        <div style="display:flex; gap:8px;">
                            <button on:click={
                                let set_all = set_all_files;
                                let set_disp = set_displayed_files;
                                let set_tags_map = set_file_tags_map;
                                let set_modal = set_show_purge_confirm;
                                move |_| {
                                    spawn_local(async move {
                                        let dbp = invoke("get_db_path", JsValue::NULL).await;
                                        if let Ok(p) = serde_wasm_bindgen::from_value::<String>(dbp.clone()) {
                                            web_sys::console::log_1(&format!("[UI] DB path={}", p).into());
                                        }
                                        let before = invoke("get_files_count", JsValue::NULL).await;
                                        if let Ok(cnt) = serde_wasm_bindgen::from_value::<u32>(before.clone()) {
                                            web_sys::console::log_1(&format!("[UI] files count before purge={}", cnt).into());
                                        }
                                        let res = invoke("purge_all_files", JsValue::NULL).await;
                                        match serde_wasm_bindgen::from_value::<u32>(res.clone()) {
                                            Ok(cnt) => web_sys::console::log_1(&format!("[UI] purge_all_files ok, count={}", cnt).into()),
                                            Err(e) => web_sys::console::error_1(&format!("[UI] purge_all_files parse error: {:?}; raw={:?}", e, res).into()),
                                        }
                                        let after = invoke("get_files_count", JsValue::NULL).await;
                                        if let Ok(cnt) = serde_wasm_bindgen::from_value::<u32>(after.clone()) {
                                            web_sys::console::log_1(&format!("[UI] files count after purge={}", cnt).into());
                                        }
                                        load_all_files(set_all, set_disp, set_tags_map).await;
                                        set_modal.set(false);
                                    });
                                }
                            }>"Confirm"</button>
                            <button on:click=move |_| set_show_purge_confirm.set(false)>"Cancel"</button>
                        </div>
                    </div>
                </div>
            })}

            {move || show_update_modal.get().then(|| view! {
                <div class="modal-overlay" on:click=move |_| set_show_update_modal.set(false)>
                    <div class="modal" on:click={|e| e.stop_propagation()}>
                        <h3>"Updates"</h3>
                        <p>{format!("Current: {}", update_current.get())}</p>
                        <p>{format!("Latest: {}", update_latest.get())}</p>
                        <Show when=move || update_has.get() fallback=move || view! { <p>"You are up to date."</p> }>
                            <div style="display:flex; gap:8px;">
                                <button on:click=move |_| {
                                    spawn_local(async move {
                                        let _ = invoke("updater_install", JsValue::NULL).await;
                                    });
                                }>
                                    "Install"
                                </button>
                            </div>
                        </Show>
                        <div style="margin-top:8px;">
                            <button on:click=move |_| set_show_update_modal.set(false)>"Close"</button>
                        </div>
                    </div>
                </div>
            })}

            {move || show_delete_tag_confirm.get().then(|| view! {
                <div class="modal-overlay" on:click=move |_| set_show_delete_tag_confirm.set(false)>
                    <div class="modal" on:click={|e| e.stop_propagation()}>
                        {move || {
                            let tid_opt = delete_target_tag_id.get();
                            let name = tid_opt.and_then(|tid| all_tags.get().iter().find(|t| t.id == tid).map(|t| t.name.clone())).unwrap_or_else(|| "".to_string());
                            view! { <h3>{format!("Delete tag '{}' ?", name)}</h3> }
                        }}
                        <p>"This will also delete its child tags and relationships."</p>
                        <div style="display:flex; gap:8px;">
                            <button on:click={
                                let set_modal = set_show_delete_tag_confirm;
                                let set_sel = set_selected_tag_ids;
                                let sel_ids = selected_tag_ids;
                                let tags_sig = all_tags;
                                let use_and = use_and_logic;
                                let set_disp = set_displayed_files;
                                let all_files_sig = all_files;
                                let set_reload = set_reload_tags_trigger;
                                let del_tid_sig = delete_target_tag_id;
                                move |_| {
                                    let maybe_id = del_tid_sig.get_untracked();
                                    if let Some(id) = maybe_id {
                                        let mut current = sel_ids.get_untracked();
                                        let all = tags_sig.get_untracked();
                                        let mut stack = vec![id];
                                        let mut subtree_ids: Vec<u32> = Vec::new();
                                        while let Some(x) = stack.pop() {
                                            subtree_ids.push(x);
                                            for t in all.iter().filter(|t| t.parent_id == Some(x)) { stack.push(t.id); }
                                        }
                                        let remove_set: std::collections::HashSet<u32> = subtree_ids.iter().copied().collect();
                                        current.retain(|tid| !remove_set.contains(tid));
                                        set_sel.set(current.clone());
                                        let logic = use_and.get_untracked();
                                        if current.is_empty() {
                                            set_disp.set(all_files_sig.get_untracked());
                                        } else {
                                            filter_files(current.clone(), logic, set_disp, all_files_sig.get_untracked());
                                        }
                                        spawn_local(async move {
                                            let args = DeleteTagArgs { id };
                                            let res = invoke("delete_tag", serde_wasm_bindgen::to_value(&args).unwrap()).await;
                                            match serde_wasm_bindgen::from_value::<()> (res.clone()) {
                                                Ok(_) => web_sys::console::log_1(&format!("[UI] delete_tag ok: {}", id).into()),
                                                Err(e) => web_sys::console::error_1(&format!("[UI] delete_tag error: {:?}; raw={:?}", e, res).into()),
                                            }
                                        });
                                        set_reload.update(|v| *v += 1);
                                        set_modal.set(false);
                                    }
                                }
                            }>"Confirm"</button>
                            <button on:click=move |_| set_show_delete_tag_confirm.set(false)>"Cancel"</button>
                        </div>
                    </div>
                </div>
            })}

            {move || batch_running.get().then(|| view! {
                <div class="overlay-blocker" style="position:fixed;top:0;left:0;right:0;bottom:0;background:rgba(0,0,0,0.55);z-index:2000;display:flex;align-items:center;justify-content:center;">
                    <div class="overlay-card">
                        <div>{format!("Recommending... {}/{}", batch_progress.get(), batch_total.get())}</div>
                        <div class="progress-bar"><div class="progress-fill" style=move || format!("width: {}%", if batch_total.get()>0 { batch_progress.get()*100 / batch_total.get() } else { 0 })></div></div>
                        <div style="margin-top:12px; display:flex; gap:8px; justify-content:right;">
                            <button on:click=move |_| set_batch_cancel.set(true) style="background:#c33; color:#fff; border:none; padding:6px 12px; border-radius:4px;">"Cancel"</button>
                        </div>
                    </div>
                </div>
            })}
        </div>
    }
}

#[component]
fn TagTree(
    tags: ReadSignal<Vec<TagInfo>>,
    selected_tag_ids: ReadSignal<Vec<u32>>,
    set_selected_tag_ids: WriteSignal<Vec<u32>>,
    use_and_logic: ReadSignal<bool>,
    set_displayed_files: WriteSignal<Vec<FileInfo>>,
    all_files: ReadSignal<Vec<FileInfo>>,
    set_show_delete_tag_confirm: WriteSignal<bool>,
    set_delete_target_tag_id: WriteSignal<Option<u32>>,
    on_toggle: impl Fn(u32) + 'static + Copy + Send,
    _set_all_tags: WriteSignal<Vec<TagInfo>>,
    dragging_tag_id: ReadSignal<Option<u32>>,
    set_dragging_tag_id: WriteSignal<Option<u32>>,
    drop_target_tag_id: ReadSignal<Option<u32>>,
    set_drop_target_tag_id: WriteSignal<Option<u32>>,
    drop_position: ReadSignal<f64>,
    set_drop_position: WriteSignal<f64>,
    set_reload_tags_trigger: WriteSignal<u32>,
) -> impl IntoView {
    let root_tags = move || {
        tags.get()
            .into_iter()
            .filter(|t| t.parent_id.is_none())
            .collect::<Vec<_>>()
    };

    view! {
        <div class="tag-tree">
            <For
                each=root_tags
                key=|tag| tag.id
                children=move |tag| {
                    view! {
                        <TagNode
                            tag=tag
                            all_tags=tags
                            selected_tag_ids=selected_tag_ids
                            set_selected_tag_ids=set_selected_tag_ids
                            use_and_logic=use_and_logic
                            set_displayed_files=set_displayed_files
                            all_files=all_files
                            set_show_delete_tag_confirm=set_show_delete_tag_confirm
                            set_delete_target_tag_id=set_delete_target_tag_id
                            on_toggle=on_toggle
                            level=0
                            dragging_tag_id=dragging_tag_id
                            set_dragging_tag_id=set_dragging_tag_id
                            drop_target_tag_id=drop_target_tag_id
                            set_drop_target_tag_id=set_drop_target_tag_id
                            drop_position=drop_position
                            set_drop_position=set_drop_position
                            set_reload_tags_trigger=set_reload_tags_trigger
                        />
                    }
                }
            />
        </div>
    }
}

#[component]
fn TagNode(
    tag: TagInfo,
    all_tags: ReadSignal<Vec<TagInfo>>,
    selected_tag_ids: ReadSignal<Vec<u32>>,
    set_selected_tag_ids: WriteSignal<Vec<u32>>,
    use_and_logic: ReadSignal<bool>,
    set_displayed_files: WriteSignal<Vec<FileInfo>>,
    all_files: ReadSignal<Vec<FileInfo>>,
    set_show_delete_tag_confirm: WriteSignal<bool>,
    set_delete_target_tag_id: WriteSignal<Option<u32>>,
    on_toggle: impl Fn(u32) + 'static + Copy + Send,
    level: usize,
    dragging_tag_id: ReadSignal<Option<u32>>,
    set_dragging_tag_id: WriteSignal<Option<u32>>,
    drop_target_tag_id: ReadSignal<Option<u32>>,
    set_drop_target_tag_id: WriteSignal<Option<u32>>,
    drop_position: ReadSignal<f64>,
    set_drop_position: WriteSignal<f64>,
    set_reload_tags_trigger: WriteSignal<u32>,
) -> AnyView {
    let tag_id = tag.id;
    let children = move || {
        all_tags.get()
            .into_iter()
            .filter(move |t| t.parent_id == Some(tag_id))
            .collect::<Vec<_>>()
    };

    let is_selected = move || selected_tag_ids.get().contains(&tag_id);
    let has_children = move || !children().is_empty();
    
    let _is_dragging = move || dragging_tag_id.get() == Some(tag_id);
    let _is_drop_target = move || drop_target_tag_id.get() == Some(tag_id);

    // Mouse down - start drag
    let on_mousedown = move |ev: web_sys::MouseEvent| {
        if ev.button() == 0 {
            if let Some(target) = ev.target() {
                if target.dyn_ref::<web_sys::HtmlInputElement>().is_some() {
                    return;
                }
                if target.dyn_ref::<web_sys::HtmlButtonElement>().is_some() {
                    return;
                }
            }
            set_dragging_tag_id.set(Some(tag_id));
            web_sys::console::log_1(&format!("🟢 Start dragging tag: {}", tag_id).into());
            ev.stop_propagation();
        }
    };

    // Mouse enter - track potential drop target
    let update_position = move |ev: &web_sys::MouseEvent| {
        if dragging_tag_id.get_untracked().is_some() {
            set_drop_target_tag_id.set(Some(tag_id));
            
            // Calculate relative position (0.0 = top, 1.0 = bottom)
            if let Some(target) = ev.current_target() {
                if let Some(element) = target.dyn_ref::<web_sys::HtmlElement>() {
                    let rect = element.get_bounding_client_rect();
                    let y = ev.client_y() as f64;
                    let top = rect.top();
                    let height = rect.height();
                    
                    if height > 0.0 {
                        let relative_y = ((y - top) / height).max(0.0).min(1.0);
                        set_drop_position.set(relative_y);
                        web_sys::console::log_1(&format!("📍 Tag {} position: {:.2} (y:{}, top:{}, height:{})", 
                            tag_id, relative_y, y, top, height).into());
                    }
                }
            }
        }
    };

    let on_mouseenter = move |ev: web_sys::MouseEvent| {
        web_sys::console::log_1(&format!("🟬 Hover over tag: {}", tag_id).into());
        update_position(&ev);
    };

    let on_mousemove = move |ev: web_sys::MouseEvent| {
        update_position(&ev);
        ev.stop_propagation();
    };

    // Visual feedback based on drag state
    let node_class = move || {
        let mut classes = vec![];
        
        if dragging_tag_id.get() == Some(tag_id) {
            classes.push("dragging");
        }
        
        if drop_target_tag_id.get() == Some(tag_id) {
            let pos = drop_position.get();
            if pos < 0.25 {
                classes.push("drop-before");
            } else if pos > 0.75 {
                classes.push("drop-after");
            } else {
                classes.push("drop-child");
            }
        }
        
        classes.join(" ")
    };

    view! {
        <div 
            class=move || format!("tag-node {}", node_class())
            style=format!("margin-left: {}px", level * 20)
        >
            <label 
                class="tag-label"
                on:mousedown=on_mousedown
                on:mouseenter=on_mouseenter
                on:mousemove=on_mousemove
            >
                <input
                    type="checkbox"
                    prop:checked=is_selected
                    on:change=move |_| on_toggle(tag_id)
                />
                <span class="tag-name" style=move || tag.color.clone().map(|c| format!("color: {}", c)).unwrap_or_default()>
                    {tag.name.clone()}
                </span>
                <button
                    class="tag-delete"
                    title="Delete Tag"
                    style="margin-left:6px; border:none; background:transparent; color:#c00; cursor:pointer;"
                    on:mousedown=move |ev: web_sys::MouseEvent| {
                        ev.stop_propagation();
                        ev.prevent_default();
                    }
                    on:click=move |ev: web_sys::MouseEvent| {
                        ev.stop_propagation();
                        ev.prevent_default();
                        set_delete_target_tag_id.set(Some(tag_id));
                        set_show_delete_tag_confirm.set(true);
                    }
                >"×"</button>
            </label>
            {move || has_children().then(|| view! {
                <div class="tag-children">
                    <For
                        each=children
                        key=|t| t.id
                        children=move |child| {
                            view! {
                                <TagNode
                                    tag=child
                                    all_tags=all_tags
                                    selected_tag_ids=selected_tag_ids
                                    set_selected_tag_ids=set_selected_tag_ids
                                    use_and_logic=use_and_logic
                                    set_displayed_files=set_displayed_files
                                    all_files=all_files
                                    set_show_delete_tag_confirm=set_show_delete_tag_confirm
                                    set_delete_target_tag_id=set_delete_target_tag_id
                                    on_toggle=on_toggle
                                    level=level + 1
                                    dragging_tag_id=dragging_tag_id
                                    set_dragging_tag_id=set_dragging_tag_id
                                    drop_target_tag_id=drop_target_tag_id
                                    set_drop_target_tag_id=set_drop_target_tag_id
                                    drop_position=drop_position
                                    set_drop_position=set_drop_position
                                    set_reload_tags_trigger=set_reload_tags_trigger
                                />
                            }
                        }
                    />
                </div>
            })}
        </div>
    }.into_any()
}

#[component]
fn FileList(
    files: impl Fn() -> Vec<DisplayFile> + 'static + Send,
    selected_file_paths: ReadSignal<Vec<String>>,
    on_toggle: impl Fn(String) + 'static + Copy + Send,
    sort_column: ReadSignal<SortColumn>,
    sort_direction: ReadSignal<SortDirection>,
    on_sort: impl Fn(SortColumn) + 'static + Copy + Send,
) -> impl IntoView {
    let sort_indicator = move |col: SortColumn| {
        if sort_column.get() == col {
            match sort_direction.get() {
                SortDirection::Asc => " ▲",
                SortDirection::Desc => " ▼",
            }
        } else {
            ""
        }
    };

    view! {
        <div class="file-list">
            <table>
                <thead>
                    <tr>
                        <th></th>
                        <th class="sortable" on:click=move |_| on_sort(SortColumn::Name)>
                            "File Name" {move || sort_indicator(SortColumn::Name)}
                        </th>
                        <th class="sortable" on:click=move |_| on_sort(SortColumn::Type)>
                            "Type" {move || sort_indicator(SortColumn::Type)}
                        </th>
                        <th class="sortable" on:click=move |_| on_sort(SortColumn::Size)>
                            "Size" {move || sort_indicator(SortColumn::Size)}
                        </th>
                        <th class="sortable" on:click=move |_| on_sort(SortColumn::Date)>
                            "Modified" {move || sort_indicator(SortColumn::Date)}
                        </th>
                        <th>"Tags"</th>
                    </tr>
                </thead>
                <tbody>
                    <For
                        each=files
                        key=|file| file.path.clone()
                        children=move |file| {
                            let file_path = file.path.clone();
                            let file_path_for_toggle = file_path.clone();
                            let file_path_for_class = file_path.clone();
                            let file_path_for_checked = file_path.clone();
                            
                            let file_path_for_dblclick = file_path.clone();
                            
                                    let tags_check = file.tags.clone();
                                    let tags_loop = file.tags.clone();
                                    
                                    view! {
                                        <tr
                                            class:selected=move || selected_file_paths.get().contains(&file_path_for_class)
                                            on:dblclick=move |_| {
                                                let path = file_path_for_dblclick.clone();
                                                spawn_local(async move {
                                                    let args = OpenFileArgs { path };
                                                    let _ = invoke("open_file", serde_wasm_bindgen::to_value(&args).unwrap()).await;
                                                });
                                            }
                                        >
                                            <td on:dblclick=|e| e.stop_propagation()>
                                                <input
                                                    type="checkbox"
                                                    checked=move || selected_file_paths.get().contains(&file_path_for_checked)
                                                    on:change=move |_| on_toggle(file_path_for_toggle.clone())
                                                />
                                            </td>
                                            <td class="file-path" title=file.path.clone()>
                                                {if file.is_directory { "📁 " } else { "" }}
                                                {file.name.clone()}
                                            </td>
                                            <td>
                                                {if file.is_directory { "Folder".to_string() } else { file.extension.clone() }}
                                            </td>
                                            <td>{format_file_size(file.size_bytes)}</td>
                                            <td>{format_timestamp(file.last_modified)}</td>
                                            <td class="file-tags">
                                                <Show
                                                    when=move || !tags_check.is_empty()
                                                    fallback=|| view! { <span class="not-in-db">"Not tagged"</span> }
                                                >
                                            {
                                                let tags_inner = tags_loop.clone();
                                                view! {
                                                    <For
                                                        each=move || tags_inner.clone()
                                                        key=|tag| tag.id
                                                        children=move |tag| {
                                                            view! {
                                                                <span class="tag-badge" style=move || tag.color.clone().map(|c| format!("background-color: {}", c)).unwrap_or_default()>
                                                                    {tag.name.clone()}
                                                                </span>
                                                            }
                                                        }
                                                    />
                                                }
                                            }
                                                </Show>
                                            </td>
                                        </tr>
                                    }
                        }
                    />
                </tbody>
            </table>
        </div>
    }
}

#[component]
fn GroupedFileList(
    files: impl Fn() -> Vec<DisplayFile> + 'static + Send,
    roots: ReadSignal<Vec<String>>,
    active_root_filter: ReadSignal<Option<String>>,
    selected_file_paths: ReadSignal<Vec<String>>,
    on_toggle: impl Fn(String) + 'static + Copy + Send + Sync,
    sort_column: ReadSignal<SortColumn>,
    sort_direction: ReadSignal<SortDirection>,
    on_sort: impl Fn(SortColumn) + 'static + Copy + Send + Sync,
    set_selected_file_paths: WriteSignal<Vec<String>>,
    last_selected_file_path: ReadSignal<Option<String>>,
    set_last_selected_file_path: WriteSignal<Option<String>>,
    _recommended_map: ReadSignal<std::collections::HashMap<u32, Vec<TagInfo>>>,
    recommended_info_map: ReadSignal<std::collections::HashMap<String, Vec<RecommendItem>>>,
    show_recommended: ReadSignal<bool>,
    all_tags: ReadSignal<Vec<TagInfo>>,
    set_all_files: WriteSignal<Vec<FileInfo>>,
    set_displayed_files: WriteSignal<Vec<FileInfo>>,
    set_file_tags_map: WriteSignal<std::collections::HashMap<u32, Vec<TagInfo>>>,
) -> impl IntoView {
    fn is_under_root(file_path: &str, root: &str) -> bool {
        let mut r = root.replace('/', "\\").to_lowercase();
        if !r.ends_with('\\') { r.push('\\'); }
        let f = file_path.replace('/', "\\").to_lowercase();
        f.starts_with(&r) || f == root.replace('/', "\\").to_lowercase()
    }
    let sort_indicator = move |col: SortColumn| {
        if sort_column.get() == col {
            match sort_direction.get() {
                SortDirection::Asc => " ▲",
                SortDirection::Desc => " ▼",
            }
        } else {
            ""
        }
    };

    view! {
        <div class="file-list">
            {move || {
                let all = files();
                let roots_vec = roots.get();
                let filter = active_root_filter.get();
                let groups: Vec<(String, Vec<DisplayFile>)> = roots_vec.into_iter().map(|r| {
                    if let Some(ref f) = filter {
                        if &r != f { return (r.clone(), Vec::<DisplayFile>::new()); }
                    }
                    let v = all
                        .iter()
                        .cloned()
                        .filter(|f| is_under_root(&f.path, &r))
                        .collect::<Vec<_>>();
                    (r, v)
                }).collect();

                let total: usize = groups.iter().map(|(_, v)| v.len()).sum();

                view! {
                    <Show
                        when=move || total == 0
                        fallback=move || {
                            let groups_clone = groups.clone();
                            view! {
                                <div>
                                    <For
                                        each=move || groups_clone.clone()
                                        key=|grp: &(String, Vec<DisplayFile>)| grp.0.clone()
                                        children=move |grp: (String, Vec<DisplayFile>)| {
                                            let r = grp.0.clone();
                                            let group_files = grp.1.clone();
                                            let group_files_value = group_files.clone();
                                            let group_paths = std::sync::Arc::new(group_files.iter().map(|f| f.path.clone()).collect::<Vec<String>>());
                                            let group_files_for_empty = group_files.clone();
                                            view! {
                                                <div class="file-group">
                                                    <div class="group-header">{r.clone()}</div>
                                                    <table>
                                                        <thead>
                                                            <tr>
                                                                <th></th>
                                                                <th class="sortable" on:click=move |_| on_sort(SortColumn::Name)>
                                                                    "File Name" {move || sort_indicator(SortColumn::Name)}
                                                                </th>
                                                                <th class="sortable" on:click=move |_| on_sort(SortColumn::Type)>
                                                                    "Type" {move || sort_indicator(SortColumn::Type)}
                                                                </th>
                                                                <th class="sortable" on:click=move |_| on_sort(SortColumn::Size)>
                                                                    "Size" {move || sort_indicator(SortColumn::Size)}
                                                                </th>
                                                                <th class="sortable" on:click=move |_| on_sort(SortColumn::Date)>
                                                                    "Modified" {move || sort_indicator(SortColumn::Date)}
                                                                </th>
                                                                <th>"Tags"</th>
                                                            </tr>
                                                        </thead>
                                                        <tbody>
                                                            <For
                                                                each=move || group_files_value.clone()
                                                                key=|file| file.path.clone()
                                                                children=move |file| {
                                                                    let file_path = file.path.clone();
                                                                    let file_path_for_toggle = file_path.clone();
                                                                    let file_path_arc = std::sync::Arc::new(file_path_for_toggle.clone());
                                                                    let file_path_for_class = file_path.clone();
                                                                    let file_path_for_checked = file_path.clone();
                                                                    let file_path_for_dblclick = file_path.clone();
                                                                    let tags_check = file.tags.clone();
                                                                    let tags_loop = file.tags.clone();
                                                                    view! {
                                                                        <tr
                                                                            class:selected=move || selected_file_paths.get().contains(&file_path_for_class)
                                                                            on:dblclick=move |_| {
                                                                                let path = file_path_for_dblclick.clone();
                                                                                spawn_local(async move {
                                                                                    let args = OpenFileArgs { path };
                                                                                    let _ = invoke("open_file", serde_wasm_bindgen::to_value(&args).unwrap()).await;
                                                                                });
                                                                            }
                                                                        >
                                                                            <td on:dblclick=|e| e.stop_propagation()>
                                                                                    <input
                                                                                        type="checkbox"
                                                                                        prop:checked=move || selected_file_paths.get().contains(&file_path_for_checked)
                                                                                        on:click={
                                                                                            let value = group_paths.clone();
                                                                                            let file_path_for_toggle_click = file_path_for_toggle.clone();
                                                                                            move |ev: web_sys::MouseEvent| {
                                                                                                let shift = ev.shift_key();
                                                                                                if shift {
                                                                                                    let anchor = last_selected_file_path.get();
                                                                                                let current = file_path_for_toggle_click.clone();
                                                                                                    let paths = (*value).clone();
                                                                                                    if let Some(a) = anchor {
                                                                                                        let i1 = paths.iter().position(|p| p == &a);
                                                                                                        let i2 = paths.iter().position(|p| p == &current);
                                                                                                        if let (Some(s1), Some(s2)) = (i1, i2) {
                                                                                                            let (s, e) = if s1 <= s2 { (s1, s2) } else { (s2, s1) };
                                                                                                            let range = paths[s..=e].to_vec();
                                                                                                            set_selected_file_paths.set(range);
                                                                                                        } else {
                                                                                                            set_selected_file_paths.set(vec![current.clone()]);
                                                                                                        }
                                                                                                    } else {
                                                                                                        set_selected_file_paths.set(vec![current.clone()]);
                                                                                                    }
                                                                                                set_last_selected_file_path.set(Some(current));
                                                                                                } else {
                                                                                                on_toggle(file_path_for_toggle_click.clone());
                                                                                                set_last_selected_file_path.set(Some(file_path_for_toggle_click.clone()));
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    />
                                                                            </td>
                                                                            <td class="file-path" title=file.path.clone()>
                                                                                {if file.is_directory { "📁 " } else { "" }}
                                                                                {file.name.clone()}
                                                                            </td>
                                                                            <td>
                                                                                {if file.is_directory { "Folder".to_string() } else { file.extension.clone() }}
                                                                            </td>
                                                                            <td>{format_file_size(file.size_bytes)}</td>
                                                                            <td>{format_timestamp(file.last_modified)}</td>
                                                                            <td class="file-tags">
                                                                                <Show
                                                                                    when=move || !tags_check.is_empty()
                                                                                    fallback=|| view! { <span class="not-in-db">"Not tagged"</span> }
                                                                                >
                                                                                    {
                                                                                        let tags_inner = tags_loop.clone();
                                                                                        view! {
                                                                                            <For
                                                                                                each=move || tags_inner.clone()
                                                                                                key=|tag| tag.id
                                                                                                children=move |tag| {
                                                                                                    view! {
                                                                                                        <span class="tag-badge" style=move || tag.color.clone().map(|c| format!("background-color: {}", c)).unwrap_or_default()>
                                                                                                            {tag.name.clone()}
                                                                                                        </span>
                                                                                                    }
                                                                                                }
                                                                                            />
                                                                                        }
                                                                                    }
                                                                                </Show>
                                                                                <Show when=move || show_recommended.get() fallback=|| view!{}>
                                                                                {
                                                                                    let fp_arc_for_recs = file_path_arc.clone();
                                                                                    let file_path_key_for_recs = file_path_for_toggle.clone();
                                                                                    view! {
                                                                                        <div style="margin-top:4px; display:flex; gap:4px; flex-wrap:wrap;">
                                                                                            <For
                                                                                                each=move || {
                                                                                                    recommended_info_map.get().get(&file_path_key_for_recs).cloned().unwrap_or_default()
                                                                                                }
                                                                                                key=|ri| ri.name.clone()
                                                                                                children=move |ri: RecommendItem| {
                                                                                                    let fp_arc_local = fp_arc_for_recs.clone();
                                                                                                    let label = if ri.source == "onnx" { format!("{} ·AI", ri.name) } else if ri.source == "llm" { format!("{} ·LLM", ri.name) } else if ri.source == "llm-vision" { format!("{} ·VL", ri.name) } else { ri.name.clone() };
                                                                                                    let title_attr = format!("score: {:.3}", ri.score);
                                                                                                    let tname = ri.name.clone();
                                                                                                    view! {
                                                                                                        <button style="background:#eee; color:#555; border:none; border-radius:10px; padding:2px 6px; cursor:pointer;"
                                                                                                            title=title_attr
                                                                                                            on:click=move |_| {
                                                                                                                let fp = (*fp_arc_local).clone();
                                                                                                                // lookup tag id by name
                                                                                                                let mut found: Option<u32> = None;
                                                                                                                for tg in all_tags.get().iter() { if tg.name == tname { found = Some(tg.id); break; } }
                                                                                                                if let Some(tid) = found {
                                                                                                                    let args = AddFileTagArgs { file_path: fp.clone(), tag_id: tid };
                                                                                                                    spawn_local(async move {
                                                                                                                        let _ = invoke("add_file_tag", serde_wasm_bindgen::to_value(&args).unwrap()).await;
                                                                                                                        // Reload to reflect DB enrollment and new tag
                                                                                                                        load_all_files(set_all_files, set_displayed_files, set_file_tags_map).await;
                                                                                                                    });
                                                                                                                }
                                                                                                            }
                                                                                                        >{label}</button>
                                                                                                    }
                                                                                                }
                                                                                            />
                                                                                        </div>
                                                                                    }
                                                                                }
                                                                                </Show>
                                                                            </td>
                                                                        </tr>
                                                                    }
                                                                }
                                                            />
                                                            {move || if group_files_for_empty.is_empty() { Some(view! { <tr><td colspan="6"><em>"No files in this root"</em></td></tr> }) } else { None }}
                                                        </tbody>
                                                    </table>
                                                </div>
                                            }
                                        }
                                    />
                                </div>
                            }
                        }
                    >
                        {
                            let all_clone = all.clone();
                            let all_value = all_clone.clone();
                            let all_paths = std::sync::Arc::new(all_clone.iter().map(|f| f.path.clone()).collect::<Vec<String>>());
                            view! {
                                <div>
                                <table>
                                    <thead>
                                        <tr>
                                            <th></th>
                                            <th class="sortable" on:click=move |_| on_sort(SortColumn::Name)>
                                                "File Name" {move || sort_indicator(SortColumn::Name)}
                                            </th>
                                            <th class="sortable" on:click=move |_| on_sort(SortColumn::Type)>
                                                "Type" {move || sort_indicator(SortColumn::Type)}
                                            </th>
                                            <th class="sortable" on:click=move |_| on_sort(SortColumn::Size)>
                                                "Size" {move || sort_indicator(SortColumn::Size)}
                                            </th>
                                            <th class="sortable" on:click=move |_| on_sort(SortColumn::Date)>
                                                "Modified" {move || sort_indicator(SortColumn::Date)}
                                            </th>
                                            <th>"Tags"</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        <For
                                            each=move || all_value.clone()
                                            key=|file| file.path.clone()
                                            children=move |file| {
                                                let file_path = file.path.clone();
                                                let file_path_for_toggle = file_path.clone();
                                                let file_path_arc2 = std::sync::Arc::new(file_path_for_toggle.clone());
                                                let file_path_for_class = file_path.clone();
                                                let file_path_for_checked = file_path.clone();
                                                let file_path_for_dblclick = file_path.clone();
                                                let tags_check = file.tags.clone();
                                                let tags_loop = file.tags.clone();
                                                view! {
                                                    <tr
                                                        class:selected=move || selected_file_paths.get().contains(&file_path_for_class)
                                                        on:dblclick=move |_| {
                                                            let path = file_path_for_dblclick.clone();
                                                            spawn_local(async move {
                                                                let args = OpenFileArgs { path };
                                                                let _ = invoke("open_file", serde_wasm_bindgen::to_value(&args).unwrap()).await;
                                                            });
                                                        }
                                                    >
                                                        <td on:dblclick=|e| e.stop_propagation()>
                                                            <input
                                                                type="checkbox"
                                                                prop:checked=move || selected_file_paths.get().contains(&file_path_for_checked)
                                                                on:click={
                                                                    let value = all_paths.clone();
                                                                    let file_path_for_toggle_click2 = file_path_for_toggle.clone();
                                                                    move |ev: web_sys::MouseEvent| {
                                                                        let shift = ev.shift_key();
                                                                        if shift {
                                                                            let anchor = last_selected_file_path.get();
                                                                            let current = file_path_for_toggle_click2.clone();
                                                                            let paths = (*value).clone();
                                                                            if let Some(a) = anchor {
                                                                                let i1 = paths.iter().position(|p| p == &a);
                                                                                let i2 = paths.iter().position(|p| p == &current);
                                                                                if let (Some(s1), Some(s2)) = (i1, i2) {
                                                                                    let (s, e) = if s1 <= s2 { (s1, s2) } else { (s2, s1) };
                                                                                    let range = paths[s..=e].to_vec();
                                                                                    set_selected_file_paths.set(range);
                                                                                } else {
                                                                                    set_selected_file_paths.set(vec![current.clone()]);
                                                                                }
                                                                            } else {
                                                                                set_selected_file_paths.set(vec![current.clone()]);
                                                                            }
                                                                            set_last_selected_file_path.set(Some(current));
                                                                        } else {
                                                                            on_toggle(file_path_for_toggle_click2.clone());
                                                                            set_last_selected_file_path.set(Some(file_path_for_toggle_click2.clone()));
                                                                        }
                                                                    }
                                                                }
                                                            />
                                                        </td>
                                                        <td class="file-path" title=file.path.clone()>
                                                            {if file.is_directory { "📁 " } else { "" }}
                                                            {file.name.clone()}
                                                        </td>
                                                        <td>
                                                            {if file.is_directory { "Folder".to_string() } else { file.extension.clone() }}
                                                        </td>
                                                        <td>{format_file_size(file.size_bytes)}</td>
                                                        <td>{format_timestamp(file.last_modified)}</td>
                                                        <td class="file-tags">
                                                            <Show
                                                                when=move || !tags_check.is_empty()
                                                                fallback=|| view! { <span class="not-in-db">"Not tagged"</span> }
                                                            >
                                                                {
                                                                    let tags_inner = tags_loop.clone();
                                                                    view! {
                                                                        <For
                                                                            each=move || tags_inner.clone()
                                                                            key=|tag| tag.id
                                                                            children=move |tag| {
                                                                                view! {
                                                                                    <span class="tag-badge" style=move || tag.color.clone().map(|c| format!("background-color: {}", c)).unwrap_or_default()>
                                                                                        {tag.name.clone()}
                                                                                    </span>
                                                                                }
                                                                            }
                                                                        />
                                                                    }
                                                                }
                                                            </Show>
                                                            <Show when=move || show_recommended.get() fallback=|| view!{}>
                                                            {
                                                                let fp_arc_for_recs = file_path_arc2.clone();
                                                                let file_path_key_for_recs2 = file_path_for_toggle.clone();
                                                                view! {
                                                                    <div style="margin-top:4px; display:flex; gap:4px; flex-wrap:wrap;">
                                                                        <For
                                                                            each=move || {
                                                                                recommended_info_map.get().get(&file_path_key_for_recs2).cloned().unwrap_or_default()
                                                                            }
                                                                            key=|ri| ri.name.clone()
                                                                            children=move |ri: RecommendItem| {
                                                                                let fp_arc_local = fp_arc_for_recs.clone();
                                                                                let label = if ri.source == "onnx" { format!("{} ·AI", ri.name) } else if ri.source == "llm" { format!("{} ·LLM", ri.name) } else if ri.source == "llm-vision" { format!("{} ·VL", ri.name) } else { ri.name.clone() };
                                                                                let title_attr = format!("score: {:.3}", ri.score);
                                                                                let tname = ri.name.clone();
                                                                                view! {
                                                                                    <button style="background:#eee; color:#555; border:none; border-radius:10px; padding:2px 6px; cursor:pointer;"
                                                                                        title=title_attr
                                                                                        on:click=move |_| {
                                                                                            let fp = (*fp_arc_local).clone();
                                                                                            let mut found: Option<u32> = None;
                                                                                            for tg in all_tags.get().iter() { if tg.name == tname { found = Some(tg.id); break; } }
                                                                                            if let Some(tid) = found {
                                                                                                let args = AddFileTagArgs { file_path: fp.clone(), tag_id: tid };
                                                                                                spawn_local(async move {
                                                                                                    let _ = invoke("add_file_tag", serde_wasm_bindgen::to_value(&args).unwrap()).await;
                                                                                                    load_all_files(set_all_files, set_displayed_files, set_file_tags_map).await;
                                                                                                });
                                                                                            }
                                                                                        }
                                                                                    >{label}</button>
                                                                                }
                                                                            }
                                                                        />
                                                                    </div>
                                                                }
                                                            }
                                                            </Show>
                                                        </td>
                                                    </tr>
                                                }
                                            }
                                        />
                                    </tbody>
                                </table>
                                </div>
                            }
                        }
                    </Show>
                }
            }}
        </div>
    }
}

// Helper functions
async fn load_tags(set_all_tags: WriteSignal<Vec<TagInfo>>) {
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

async fn load_all_files(
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

fn filter_files(
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
#[derive(Clone, Debug, Deserialize)]
struct RecommendItem { name: String, score: f32, source: String }

#[derive(Clone, Debug, Deserialize)]
struct UpdateInfo { current: String, latest: Option<String>, has_update: bool }
