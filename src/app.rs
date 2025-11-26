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
}

// Full file info for files in database (with hash)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct FileInfo {
    id: u32,
    path: String,
    content_hash: String,
    size_bytes: u64,
    last_modified: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct TagInfo {
    id: u32,
    name: String,
    parent_id: Option<u32>,
    color: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct FileWithTags {
    file: FileInfo,
    tags: Vec<TagInfo>,
}

#[derive(Serialize, Deserialize)]
struct SetAlwaysOnTopArgs {
    always_on_top: bool,
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
struct SaveWindowStateArgs {
    width: f64,
    height: f64,
    x: f64,
    y: f64,
    pinned: bool,
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
    let (root_directory, set_root_directory) = signal(None::<String>);
    let (scanned_files, set_scanned_files) = signal(Vec::<FileListItem>::new());
    let (all_files, set_all_files) = signal(Vec::<FileInfo>::new());
    let (all_tags, set_all_tags) = signal(Vec::<TagInfo>::new());
    let (selected_tag_ids, set_selected_tag_ids) = signal(Vec::<u32>::new());
    let (use_and_logic, set_use_and_logic) = signal(true);
    let (displayed_files, set_displayed_files) = signal(Vec::<FileInfo>::new());
    let (file_tags_map, set_file_tags_map) = signal(std::collections::HashMap::<u32, Vec<TagInfo>>::new());
    let (selected_file_paths, set_selected_file_paths) = signal(Vec::<String>::new());
    let (is_pinned, set_is_pinned) = signal(false);
    let (scanning, set_scanning) = signal(false);
    let (show_add_tag_dialog, set_show_add_tag_dialog) = signal(false);
    let (new_tag_name, set_new_tag_name) = signal(String::new());
    let (new_tag_parent, set_new_tag_parent) = signal(None::<u32>);
    let (new_tag_input_sidebar, set_new_tag_input_sidebar) = signal(String::new());
    
    // Drag and drop state
    let (dragging_tag_id, set_dragging_tag_id) = signal(None::<u32>);
    let (drop_target_tag_id, set_drop_target_tag_id) = signal(None::<u32>);
    let (drop_position, set_drop_position) = signal(0.5f64); // 0.0=top, 1.0=bottom
    let (reload_tags_trigger, set_reload_tags_trigger) = signal(0u32);

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
                            spawn_local(async move {
                                let args = MoveTagArgs {
                                    id: dragged_id,
                                    new_parent_id: Some(target_id),
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
            // Load root directory
            let root: Option<String> = serde_wasm_bindgen::from_value(
                invoke("get_root_directory", JsValue::NULL).await
            ).unwrap_or(None);
            set_root_directory.set(root);

            // Load tags
            load_tags(set_all_tags).await;

            // Load all files
            load_all_files(set_all_files, set_displayed_files, set_file_tags_map).await;

            // Load window state
            let state_value = invoke("load_window_state", JsValue::NULL).await;
            if let Ok(Some(state)) = serde_wasm_bindgen::from_value::<Option<serde_json::Value>>(state_value) {
                if let Some(pinned) = state.get("pinned").and_then(|v| v.as_bool()) {
                    set_is_pinned.set(pinned);
                }
            }
        });
    });

    let select_directory = move |_| {
        spawn_local(async move {
            let path_val = invoke("select_root_directory", JsValue::NULL).await;
            
            if let Ok(path) = serde_wasm_bindgen::from_value::<String>(path_val) {
                set_root_directory.set(Some(path.clone()));
                
                // Automatically trigger scan after selecting directory
                set_scanning.set(true);
                web_sys::console::log_1(&"Auto-scanning after selection...".into());
                let args = ScanFilesArgs { root_path: path };
                
                // Tauri unwraps Result automatically, so expect Vec<FileListItem> directly
                let scan_result = match serde_wasm_bindgen::from_value::<Vec<FileListItem>>(
                    invoke("scan_files", serde_wasm_bindgen::to_value(&args).unwrap()).await
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
                }
            }
        });
    };

    let scan_directory = move |_| {
        let root = root_directory.get();
        if let Some(path) = root {
            set_scanning.set(true);
            spawn_local(async move {
                web_sys::console::log_1(&"Invoking scan_files...".into());
                let args = ScanFilesArgs { root_path: path };
                
                // Tauri unwraps Result automatically, so expect Vec<FileListItem> directly
                let result = match serde_wasm_bindgen::from_value::<Vec<FileListItem>>(
                    invoke("scan_files", serde_wasm_bindgen::to_value(&args).unwrap()).await
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
                }
            });
        }
    };

    let toggle_pin = move |_| {
        let new_pinned = !is_pinned.get();
        set_is_pinned.set(new_pinned);
        spawn_local(async move {
            let args = SetAlwaysOnTopArgs { always_on_top: new_pinned };
            let _ = invoke("set_always_on_top", serde_wasm_bindgen::to_value(&args).unwrap()).await;
        });
    };

    let close = move |_| {
        spawn_local(async move {
            let _ = invoke("close_window", JsValue::NULL).await;
        });
    };

    let start_drag = move |_| {
        spawn_local(async move {
            let _ = invoke("start_drag", JsValue::NULL).await;
        });
    };

    let toggle_tag_selection = move |tag_id: u32| {
        let mut current = selected_tag_ids.get();
        if let Some(pos) = current.iter().position(|&id| id == tag_id) {
            current.remove(pos);
        } else {
            current.push(tag_id);
        }
        set_selected_tag_ids.set(current.clone());
        
        // Filter files
        filter_files(current, use_and_logic.get(), set_displayed_files, all_files.get());
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

    let add_tag_to_selected_files = move |tag_id: u32| {
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
            <div class="header" on:mousedown=move |e| {
                // Only start drag if not clicking on buttons
                let target = e.target();
                if let Some(element) = target.and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok()) {
                    let tag_name = element.tag_name().to_lowercase();
                    if tag_name != "button" {
                        start_drag(e);
                    }
                }
            }>
                <h1>"TagMe"</h1>
                <div class="header-buttons">
                    <button on:click=toggle_pin class="header-btn" title="Pin">
                        {move || if is_pinned.get() { "📌" } else { "📍" }}
                    </button>
                    <button on:click=close class="header-btn" title="Close">"×"</button>
                </div>
            </div>

            <div class="toolbar">
                <button on:click=select_directory>"Select Root Directory"</button>
                {move || root_directory.get().map(|path| view! {
                    <span class="root-path">{path}</span>
                })}
                <button on:click=scan_directory disabled=move || root_directory.get().is_none()>
                    {move || if scanning.get() { "Scanning..." } else { "Scan Files" }}
                </button>
            </div>

            <div class="main-content">
                <div class="left-panel">
                    <div class="panel-header">
                        <h2>"Tags"</h2>
                        <button on:click=move |_| set_show_add_tag_dialog.set(true)>"+"</button>
                    </div>
                    <TagTree
                        tags=all_tags
                        selected_tag_ids=selected_tag_ids
                        on_toggle=toggle_tag_selection
                        set_all_tags=set_all_tags
                        dragging_tag_id=dragging_tag_id
                        set_dragging_tag_id=set_dragging_tag_id
                        drop_target_tag_id=drop_target_tag_id
                        set_drop_target_tag_id=set_drop_target_tag_id
                        drop_position=drop_position
                        set_drop_position=set_drop_position
                        set_reload_tags_trigger=set_reload_tags_trigger
                    />
                </div>

                <div class="center-panel">
                    <div class="panel-header">
                        <h2>"Files"</h2>
                        <div class="file-controls">
                            <button on:click=show_all>"Show All"</button>
                            <button on:click=toggle_and_or>
                                {move || if use_and_logic.get() { "Filter: AND" } else { "Filter: OR" }}
                            </button>
                        </div>
                    </div>
                    <FileList
                        scanned_files=scanned_files
                        db_files=displayed_files
                        file_tags_map=file_tags_map
                        selected_file_paths=selected_file_paths
                        on_toggle=toggle_file_selection
                    />
                </div>

                <div class="right-sidebar">
                    <div class="panel-header">
                        <h2>"File Tags"</h2>
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
                                                                    // Add tag to all selected files
                                                                    for p in &ps {
                                                                        let pc = p.clone();
                                                                        spawn_local(async move {
                                                                            let args = AddFileTagArgs { file_path: pc, tag_id: tid };
                                                                            let _ = invoke("add_file_tag", serde_wasm_bindgen::to_value(&args).unwrap()).await;
                                                                        });
                                                                    }
                                                                } else {
                                                                    // Remove tag from all selected files
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
                    <div class="modal" on:click=|e| e.stop_propagation()>
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
        </div>
    }
}

#[component]
fn TagTree(
    tags: ReadSignal<Vec<TagInfo>>,
    selected_tag_ids: ReadSignal<Vec<u32>>,
    on_toggle: impl Fn(u32) + 'static + Copy + Send,
    set_all_tags: WriteSignal<Vec<TagInfo>>,
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
    
    let is_dragging = move || dragging_tag_id.get() == Some(tag_id);
    let is_drop_target = move || drop_target_tag_id.get() == Some(tag_id);

    // Mouse down - start drag
    let on_mousedown = move |ev: web_sys::MouseEvent| {
        if ev.button() == 0 { // Left click only
            set_dragging_tag_id.set(Some(tag_id));
            web_sys::console::log_1(&format!("🟢 Start dragging tag: {}", tag_id).into());
            ev.prevent_default();
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
                    checked=is_selected
                    on:change=move |_| on_toggle(tag_id)
                />
                <span class="tag-name" style=move || tag.color.clone().map(|c| format!("color: {}", c)).unwrap_or_default()>
                    {tag.name.clone()}
                </span>
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
    scanned_files: ReadSignal<Vec<FileListItem>>,
    db_files: ReadSignal<Vec<FileInfo>>,
    file_tags_map: ReadSignal<std::collections::HashMap<u32, Vec<TagInfo>>>,
    selected_file_paths: ReadSignal<Vec<String>>,
    on_toggle: impl Fn(String) + 'static + Copy + Send,
) -> impl IntoView {
    view! {
        <div class="file-list">
            <table>
                <thead>
                    <tr>
                        <th></th>
                        <th>"File Path"</th>
                        <th>"Size"</th>
                        <th>"Modified"</th>
                        <th>"Tags"</th>
                    </tr>
                </thead>
                <tbody>
                    // Show scanned files first (not yet in DB)
                    <For
                        each=move || scanned_files.get()
                        key=|file| file.path.clone()
                        children=move |file| {
                            let file_path = file.path.clone();
                            let file_path_for_toggle = file_path.clone();
                            let file_path_for_class = file_path.clone();
                            let file_path_for_checked = file_path.clone();
                            
                            view! {
                                <tr class:selected=move || selected_file_paths.get().contains(&file_path_for_class)>
                                    <td>
                                        <input
                                            type="checkbox"
                                            checked=move || selected_file_paths.get().contains(&file_path_for_checked)
                                            on:change=move |_| on_toggle(file_path_for_toggle.clone())
                                        />
                                    </td>
                                    <td class="file-path" title=file.path.clone()>{file.path.clone()}</td>
                                    <td>{format_file_size(file.size_bytes)}</td>
                                    <td>{format_timestamp(file.last_modified)}</td>
                                    <td class="file-tags">
                                        <span class="not-in-db">"Not tagged yet"</span>
                                    </td>
                                </tr>
                            }
                        }
                    />
                    
                    // Show DB files (already tagged)
                    <For
                        each=move || db_files.get()
                        key=|file| file.id
                        children=move |file| {
                            let file_path = file.path.clone();
                            let file_path_for_toggle = file_path.clone();
                            let file_path_for_class = file_path.clone();
                            let file_path_for_checked = file_path.clone();
                            let file_id = file.id;
                            let file_tags = move || file_tags_map.get().get(&file_id).cloned().unwrap_or_default();
                            
                            view! {
                                <tr class:selected=move || selected_file_paths.get().contains(&file_path_for_class)>
                                    <td>
                                        <input
                                            type="checkbox"
                                            checked=move || selected_file_paths.get().contains(&file_path_for_checked)
                                            on:change=move |_| on_toggle(file_path_for_toggle.clone())
                                        />
                                    </td>
                                    <td class="file-path" title=file.path.clone()>{file.path.clone()}</td>
                                    <td>{format_file_size(file.size_bytes)}</td>
                                    <td>{format_timestamp(file.last_modified)}</td>
                                    <td class="file-tags">
                                        <For
                                            each=file_tags
                                            key=|tag| tag.id
                                            children=move |tag| {
                                                view! {
                                                    <span class="tag-badge" style=move || tag.color.clone().map(|c| format!("background-color: {}", c)).unwrap_or_default()>
                                                        {tag.name.clone()}
                                                    </span>
                                                }
                                            }
                                        />
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

// Helper functions
async fn load_tags(set_all_tags: WriteSignal<Vec<TagInfo>>) {
    web_sys::console::log_1(&"Loading tags...".into());
    let tags_val = invoke("get_all_tags", JsValue::NULL).await;
    
    match serde_wasm_bindgen::from_value::<Vec<TagInfo>>(tags_val) {
        Ok(tags) => {
            web_sys::console::log_1(&format!("Loaded {} tags", tags.len()).into());
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
        let args = FilterFilesByTagsArgs {
            tag_ids,
            use_and_logic: use_and,
        };
        let result_val = invoke("filter_files_by_tags", serde_wasm_bindgen::to_value(&args).unwrap()).await;
        
        if let Ok(files) = serde_wasm_bindgen::from_value::<Vec<FileInfo>>(result_val) {
            set_displayed_files.set(files);
        }
    });
}
