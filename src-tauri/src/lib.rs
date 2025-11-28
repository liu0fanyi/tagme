use tauri::{Emitter, Manager};
use tauri_plugin_dialog::DialogExt;

use notify::{Event, RecursiveMode, Watcher};
use std::sync::{Arc, Mutex};

mod ai;
mod db;

// Global file watcher state
static WATCHERS: Mutex<Vec<Arc<Mutex<notify::RecommendedWatcher>>>> = Mutex::new(Vec::new());

// Window management commands
#[tauri::command]
fn set_always_on_top(window: tauri::Window, always_on_top: bool) {
    println!("Setting always on top to: {}", always_on_top);
    if let Err(e) = window.set_always_on_top(always_on_top) {
        println!("Error setting always on top: {}", e);
    }
}

#[tauri::command]
fn close_window(window: tauri::Window) {
    let _ = window.close();
}

#[tauri::command]
fn minimize_window(window: tauri::Window) {
    let _ = window.minimize();
}

#[tauri::command]
fn start_drag(window: tauri::Window) {
    let _ = window.start_dragging();
}

#[tauri::command]
fn toggle_maximize(window: tauri::Window) {
    if let Ok(is_maximized) = window.is_maximized() {
        if is_maximized {
            let _ = window.unmaximize();
        } else {
            let _ = window.maximize();
        }
    }
}

// Root directory commands
#[tauri::command]
async fn select_root_directory(app_handle: tauri::AppHandle) -> Result<Option<String>, String> {
    let dialog = app_handle.dialog().file();

    if let Some(file_path) = dialog.blocking_pick_folder() {
        if let Some(path) = file_path.as_path() {
            if let Some(path_str) = path.to_str() {
                db::add_root_directory(&app_handle, path_str.to_string())
                    .map_err(|e| e.to_string())?;
                return Ok(Some(path_str.to_string()));
            }
        }
        Err("Invalid path encoding".to_string())
    } else {
        Ok(None)
    }
}

#[tauri::command]
fn get_root_directory(app_handle: tauri::AppHandle) -> Option<String> {
    db::get_root_directory(&app_handle).ok().flatten()
}

#[tauri::command]
fn get_root_directories(app_handle: tauri::AppHandle) -> Result<Vec<String>, String> {
    db::get_root_directories(&app_handle).map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_root_directory(app_handle: tauri::AppHandle, path: String) -> Result<(), String> {
    db::remove_root_directory(&app_handle, path).map_err(|e| e.to_string())
}


#[tauri::command]
fn purge_files_under_root(app_handle: tauri::AppHandle, path: String) -> Result<u32, String> {
    db::delete_files_under_root(&app_handle, path)
        .map(|n| n as u32)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn purge_all_files(app_handle: tauri::AppHandle) -> Result<u32, String> {
    eprintln!("[TAURI] purge_all_files called");
    match db::purge_all_files(&app_handle) {
        Ok(n) => {
            eprintln!("[TAURI] purge_all_files deleted {} rows", n);
            Ok(n as u32)
        }
        Err(e) => {
            eprintln!("[TAURI] purge_all_files error: {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
fn get_db_path(app_handle: tauri::AppHandle) -> String {
    db::get_db_path_string(&app_handle)
}

#[tauri::command]
fn get_files_count(app_handle: tauri::AppHandle) -> Result<u32, String> {
    db::get_files_count(&app_handle).map_err(|e| e.to_string())
}

// File scanning commands
#[tauri::command]
fn scan_files(
    app_handle: tauri::AppHandle,
    root_path: String,
) -> Result<Vec<db::FileListItem>, String> {
    eprintln!(
        "🎯 [TAURI] scan_files command called with path: {}",
        root_path
    );

    // Prune missing files first to keep DB in sync
    if let Err(e) = db::prune_missing_files(&app_handle) {
        eprintln!("⚠️ [TAURI] Warning: Failed to prune missing files: {}", e);
    }

    let result = db::scan_directory_lightweight(root_path).map_err(|e| {
        let err_msg = e.to_string();
        eprintln!("❌ [TAURI] scan_files failed: {}", err_msg);
        err_msg
    });
    if result.is_ok() {
        eprintln!("✅ [TAURI] scan_files completed successfully");
    }
    result
}

#[tauri::command]
fn scan_files_multi(
    app_handle: tauri::AppHandle,
    root_paths: Vec<String>,
) -> Result<Vec<db::FileListItem>, String> {
    eprintln!(
        "🎯 [TAURI] scan_files_multi command called with paths: {:?}",
        root_paths
    );
    if let Err(e) = db::prune_missing_files(&app_handle) {
        eprintln!("⚠️ [TAURI] Warning: Failed to prune missing files: {}", e);
    }
    let result = db::scan_directories_lightweight(root_paths).map_err(|e| e.to_string());
    if result.is_ok() {
        eprintln!("✅ [TAURI] scan_files_multi completed successfully");
    }
    result
}

// File watching commands
#[tauri::command]
fn start_watching(app_handle: tauri::AppHandle, root_path: String) -> Result<(), String> {
    use notify::EventKind;

    eprintln!("🔍 [TAURI] start_watching called for: {}", root_path);

    let path = std::path::PathBuf::from(root_path.clone());
    let app = app_handle.clone();

    let watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
        match res {
            Ok(event) => {
                eprintln!("📬 [WATCHER] Event received: {:?}", event);
                // Only emit events for Create, Modify, and Remove
                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                        eprintln!(
                            "📁 [WATCHER] File change detected: {:?}, paths: {:?}",
                            event.kind, event.paths
                        );
                        match app.emit("file-system-change", ()) {
                            Ok(_) => eprintln!("✅ [WATCHER] Event emitted successfully"),
                            Err(e) => eprintln!("❌ [WATCHER] Failed to emit event: {:?}", e),
                        }
                    }
                    _ => {
                        eprintln!("⏭️ [WATCHER] Ignoring event kind: {:?}", event.kind);
                    }
                }
            }
            Err(e) => eprintln!("❌ [WATCHER] Error: {:?}", e),
        }
    })
    .map_err(|e| e.to_string())?;

    let watcher_arc = Arc::new(Mutex::new(watcher));

    // Start watching
    watcher_arc
        .lock()
        .unwrap()
        .watch(&path, RecursiveMode::NonRecursive)
        .map_err(|e| e.to_string())?;

    WATCHERS.lock().unwrap().push(watcher_arc);

    eprintln!("✅ [TAURI] File watching started for: {}", root_path);
    eprintln!("📊 [TAURI] Watching mode: NonRecursive");
    Ok(())
}

#[tauri::command]
fn stop_watching() -> Result<(), String> {
    eprintln!("🛑 [TAURI] stop_watching called");

    let mut list = WATCHERS.lock().unwrap();
    list.clear();

    eprintln!("✅ [TAURI] File watching stopped");
    Ok(())
}

#[tauri::command]
fn start_watching_multi(
    app_handle: tauri::AppHandle,
    root_paths: Vec<String>,
) -> Result<(), String> {
    for p in root_paths {
        let _ = start_watching(app_handle.clone(), p);
    }
    Ok(())
}

#[tauri::command]
fn get_all_files(app_handle: tauri::AppHandle) -> Result<Vec<db::FileInfo>, String> {
    db::get_all_files(&app_handle).map_err(|e| e.to_string())
}

// Tag CRUD commands
#[tauri::command]
fn create_tag(
    app_handle: tauri::AppHandle,
    name: String,
    parent_id: Option<u32>,
    color: Option<String>,
) -> Result<u32, String> {
    db::create_tag(&app_handle, name, parent_id, color).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_all_tags(app_handle: tauri::AppHandle) -> Result<Vec<db::TagInfo>, String> {
    db::get_all_tags(&app_handle).map_err(|e| e.to_string())
}

#[tauri::command]
fn update_tag(
    app_handle: tauri::AppHandle,
    id: u32,
    name: String,
    color: Option<String>,
) -> Result<(), String> {
    db::update_tag(&app_handle, id, name, color).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_tag(app_handle: tauri::AppHandle, id: u32) -> Result<(), String> {
    db::delete_tag(&app_handle, id).map_err(|e| e.to_string())
}

#[tauri::command]
fn move_tag(
    app_handle: tauri::AppHandle,
    id: u32,
    new_parent_id: Option<u32>,
    target_position: i32,
) -> Result<(), String> {
    db::move_tag(&app_handle, id, new_parent_id, target_position).map_err(|e| e.to_string())
}

// File-tag relationship commands
#[tauri::command]
fn add_file_tag(
    app_handle: tauri::AppHandle,
    file_path: String,
    tag_id: u32,
) -> Result<(), String> {
    db::add_file_tag(&app_handle, file_path, tag_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_file_tag(app_handle: tauri::AppHandle, file_id: u32, tag_id: u32) -> Result<(), String> {
    db::remove_file_tag(&app_handle, file_id, tag_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_file_tags(app_handle: tauri::AppHandle, file_id: u32) -> Result<Vec<db::TagInfo>, String> {
    db::get_file_tags(&app_handle, file_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn filter_files_by_tags(
    app_handle: tauri::AppHandle,
    tag_ids: Vec<u32>,
    use_and_logic: bool,
) -> Result<Vec<db::FileInfo>, String> {
    db::get_files_by_tags(&app_handle, tag_ids, use_and_logic).map_err(|e| e.to_string())
}

// Window state commands
#[tauri::command]
fn save_window_state(
    app_handle: tauri::AppHandle,
    width: f64,
    height: f64,
    x: f64,
    y: f64,
    pinned: bool,
) {
    let _ = db::save_window_state(&app_handle, width, height, x, y, pinned);
}

#[tauri::command]
fn load_window_state(app_handle: tauri::AppHandle) -> Option<db::WindowState> {
    db::load_window_state(&app_handle).ok().flatten()
}

#[tauri::command]
fn open_file(path: String) -> Result<(), String> {
    eprintln!("📂 Opening file: {}", path);

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct RecommendItem {
    name: String,
    score: f32,
    source: String,
}

#[tauri::command]
async fn generate_tags_llm(
    title: String,
    labels: Vec<String>,
    top_k: usize,
    threshold: f32,
    base_url: Option<String>,
    model: Option<String>,
) -> Result<Vec<RecommendItem>, String> {
    use async_openai::config::OpenAIConfig;
    use async_openai::types::{
        ChatCompletionRequestMessage, ChatCompletionResponseFormat, CreateChatCompletionRequestArgs,
    };
    use async_openai::types::{
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
    };
    use async_openai::Client;

    let api_key = std::env::var("SILICONFLOW_API_KEY")
        .map_err(|_| "SILICONFLOW_API_KEY not set".to_string())?;
    let base = base_url.unwrap_or_else(|| {
        std::env::var("LLM_BASE_URL")
            .unwrap_or_else(|_| "https://api.siliconflow.cn/v1".to_string())
    });
    let model_name = model.unwrap_or_else(|| {
        std::env::var("LLM_MODEL").unwrap_or_else(|_| "deepseek-ai/DeepSeek-V3.2-Exp".to_string())
    });

    let cfg = OpenAIConfig::new()
        .with_api_base(base)
        .with_api_key(api_key);
    let client = Client::with_config(cfg);

    // Pre-filter labels by simple rule to reduce prompt size
    let lname = title.to_lowercase();
    let tokens: Vec<&str> = lname
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
    let mut scored: Vec<(String, i32)> = Vec::new();
    for l in &labels {
        let ln = l.to_lowercase();
        let mut s = 0;
        if !ln.is_empty() {
            if lname.contains(&ln) {
                s += 10;
            }
            if tokens.iter().any(|w| *w == ln) {
                s += 8;
            }
            if lname.starts_with(&ln) || lname.ends_with(&ln) {
                s += 4;
            }
        }
        if s > 0 {
            scored.push((l.clone(), s));
        } else {
            scored.push((l.clone(), 0));
        }
    }
    scored.sort_by(|a, b| b.1.cmp(&a.1));
    let max_send = core::cmp::min(scored.len(), 20);
    let labels_to_send: Vec<String> = scored.into_iter().take(max_send).map(|(l, _)| l).collect();

    // System instructions for title (text) tagging
    let sys = ChatCompletionRequestMessage::System(
        ChatCompletionRequestSystemMessageArgs::default()
            .content("你是一个文本标题标签推荐助手。输入是文件标题（纯文本），只从已存在的标签列表中挑选，尽可能返回多个（最多 top_k），并给出置信度。严格输出 JSON：{\"items\":[{\"name\":string,\"confidence\":number}]}. 不要创建新标签、不要包含除 JSON 外的任何文本。")
            .build().map_err(|e| e.to_string())?
    );
    let user_content = format!(
        "title: {}\nlabels: {}\n要求：只从 labels 中选择，最多 {} 个。",
        title,
        serde_json::to_string(&labels_to_send).unwrap_or_default(),
        top_k
    );
    let user = ChatCompletionRequestMessage::User(
        ChatCompletionRequestUserMessageArgs::default()
            .content(user_content)
            .build()
            .map_err(|e| e.to_string())?,
    );
    let req = CreateChatCompletionRequestArgs::default()
        .model(model_name)
        .messages(vec![sys, user])
        .build()
        .map_err(|e| e.to_string())?;

    let t0 = std::time::Instant::now();
    let resp = client.chat().create(req).await.map_err(|e| e.to_string())?;
    println!(
        "[LLM] title='{}' latency={}ms, labels_sent={}",
        title,
        t0.elapsed().as_millis(),
        labels_to_send.len()
    );
    let mut out: Vec<RecommendItem> = Vec::new();
    if let Some(choice) = resp.choices.first() {
        if let Some(content) = &choice.message.content {
            // Try direct JSON parse; if it fails, attempt to extract JSON object from fenced code or plain text
            let raw = content.clone();
            let try_parse = |s: &str| -> Result<serde_json::Value, String> {
                serde_json::from_str::<serde_json::Value>(s).map_err(|e| e.to_string())
            };
            let v = match try_parse(&raw) {
                Ok(val) => val,
                Err(_) => {
                    // Strip code fences ```json ... ``` or ``` ... ```
                    let mut s = raw.replace("```json", "").replace("```", "");
                    // Trim leading non-json noise
                    if let (Some(start), Some(end)) = (s.find('{'), s.rfind('}')) {
                        s = s[start..=end].to_string();
                    }
                    match try_parse(&s) {
                        Ok(val2) => val2,
                        Err(_) => serde_json::json!({"items": []}),
                    }
                }
            };
            if let Some(items) = v.get("items").and_then(|x| x.as_array()) {
                for it in items {
                    let name = it
                        .get("name")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string();
                    if !labels.iter().any(|l| l == &name) {
                        continue;
                    }
                    let confidence =
                        it.get("confidence").and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
                    out.push(RecommendItem {
                        name,
                        score: confidence,
                        source: "llm".to_string(),
                    });
                }
            }
        }
    }
    // Top-k & threshold filter
    out.sort_by(|a, b| b.score.total_cmp(&a.score));
    let filtered: Vec<RecommendItem> = out
        .into_iter()
        .filter(|x| x.score >= threshold)
        .take(top_k)
        .collect();
    Ok(filtered)
}

#[tauri::command]
async fn generate_image_tags_llm(
    image_path: String,
    labels: Vec<String>,
    top_k: usize,
    threshold: f32,
    base_url: Option<String>,
    model: Option<String>,
) -> Result<Vec<RecommendItem>, String> {
    use async_openai::config::OpenAIConfig;
    use async_openai::types::{
        ChatCompletionRequestMessage, ChatCompletionRequestMessageContentPart,
        ChatCompletionRequestMessageContentPartImageArgs,
        ChatCompletionRequestMessageContentPartTextArgs, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs, ImageUrlArgs,
    };
    use async_openai::Client;

    let api_key = std::env::var("SILICONFLOW_API_KEY")
        .map_err(|_| "SILICONFLOW_API_KEY not set".to_string())?;
    let base = base_url.unwrap_or_else(|| {
        std::env::var("LLM_BASE_URL")
            .unwrap_or_else(|_| "https://api.siliconflow.cn/v1".to_string())
    });
    let model_name = model.unwrap_or_else(|| {
        std::env::var("LLM_MODEL").unwrap_or_else(|_| "deepseek-ai/deepseek-vl2".to_string())
    });

    let bytes = std::fs::read(&image_path).map_err(|e| e.to_string())?;
    let mime = {
        let p = std::path::Path::new(&image_path);
        match p
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
            .as_deref()
        {
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("png") => "image/png",
            Some("webp") => "image/webp",
            _ => "image/jpeg",
        }
    };
    let data_url = {
        use base64::engine::general_purpose::STANDARD;
        use base64::Engine;
        format!("data:{};base64,{}", mime, STANDARD.encode(&bytes))
    };

    let cfg = OpenAIConfig::new()
        .with_api_base(base)
        .with_api_key(api_key);
    let client = Client::with_config(cfg);

    let sys = ChatCompletionRequestMessage::System(
        ChatCompletionRequestSystemMessageArgs::default()
            .content("你是一个图片标签推荐助手。只从已存在的标签列表中挑选，尽可能返回多个（最多 top_k），并给出置信度。严格输出 JSON：{\"items\":[{\"name\":string,\"confidence\":number}]}. 不要创建新标签、不要包含除 JSON 外的任何文本。")
            .build().map_err(|e| e.to_string())?
    );

    let text_part = ChatCompletionRequestMessageContentPart::Text(
        ChatCompletionRequestMessageContentPartTextArgs::default()
            .text(format!(
                "labels: {}\n最多选择 {} 个，只从 labels 中选择。",
                serde_json::to_string(&labels).unwrap_or_default(),
                top_k
            ))
            .build()
            .unwrap(),
    );
    let image_part = ChatCompletionRequestMessageContentPart::Image(
        ChatCompletionRequestMessageContentPartImageArgs::default()
            .image_url(ImageUrlArgs::default().url(data_url).build().unwrap())
            .build()
            .unwrap(),
    );
    let user = ChatCompletionRequestMessage::User(
        ChatCompletionRequestUserMessageArgs::default()
            .content(vec![text_part, image_part])
            .build()
            .map_err(|e| e.to_string())?,
    );

    let req = CreateChatCompletionRequestArgs::default()
        .model(model_name)
        .temperature(0.0)
        .messages(vec![sys, user])
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client.chat().create(req).await.map_err(|e| e.to_string())?;
    let mut out: Vec<RecommendItem> = Vec::new();
    if let Some(choice) = resp.choices.first() {
        if let Some(content) = &choice.message.content {
            let raw = content.clone();
            let v = match serde_json::from_str::<serde_json::Value>(&raw) {
                Ok(val) => val,
                Err(_) => {
                    let mut s = raw.replace("```json", "").replace("```", "");
                    if let (Some(start), Some(end)) = (s.find('{'), s.rfind('}')) {
                        s = s[start..=end].to_string();
                    }
                    serde_json::from_str::<serde_json::Value>(&s)
                        .unwrap_or_else(|_| serde_json::json!({"items": []}))
                }
            };
            if let Some(items) = v.get("items").and_then(|x| x.as_array()) {
                let mut allowed = std::collections::HashSet::new();
                for l in &labels {
                    allowed.insert(l.to_lowercase());
                }
                for it in items {
                    let name = it
                        .get("name")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    if !allowed.contains(&name.to_lowercase()) {
                        continue;
                    }
                    let confidence =
                        it.get("confidence").and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
                    out.push(RecommendItem {
                        name,
                        score: confidence,
                        source: "llm-vision".to_string(),
                    });
                }
            }
        }
    }
    if out.is_empty() {
        let stem = std::path::Path::new(&image_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        let tokens: Vec<&str> = stem
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .collect();
        let mut scored: Vec<(String, i32)> = Vec::new();
        for l in &labels {
            let ln = l.to_lowercase();
            let mut s = 0;
            if !ln.is_empty() {
                if stem.contains(&ln) { s += 10; }
                if tokens.iter().any(|w| *w == ln) { s += 8; }
                if stem.starts_with(&ln) || stem.ends_with(&ln) { s += 4; }
            }
            if s > 0 { scored.push((l.clone(), s)); }
        }
        scored.sort_by(|a, b| b.1.cmp(&a.1));
        for (name, _) in scored.into_iter().take(top_k) {
            out.push(RecommendItem { name, score: 0.0, source: "rule".to_string() });
        }
    }
    out.sort_by(|a, b| b.score.total_cmp(&a.score));
    Ok(out
        .into_iter()
        .filter(|x| x.score >= threshold)
        .take(top_k)
        .collect())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = app
                .get_webview_window("main")
                .expect("no main window")
                .set_focus();
        }))
        .plugin(tauri_plugin_dialog::init())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_) = event {
                let win = window.clone();
                std::thread::spawn(move || {
                    // Don't save window size if maximized to prevent incorrect restoration
                    if let Ok(is_maximized) = win.is_maximized() {
                        if is_maximized {
                            return;
                        }
                    }

                    if let Ok(factor) = win.scale_factor() {
                        if let (Ok(pos), Ok(size)) = (win.outer_position(), win.inner_size()) {
                            let logical_pos = pos.to_logical::<f64>(factor);
                            let logical_size = size.to_logical::<f64>(factor);
                            let app_handle = win.app_handle();
                            let pinned = if let Ok(Some(state)) = db::load_window_state(app_handle)
                            {
                                state.pinned
                            } else {
                                false
                            };

                            let _ = db::save_window_state(
                                app_handle,
                                logical_size.width,
                                logical_size.height,
                                logical_pos.x,
                                logical_pos.y,
                                pinned,
                            );
                        }
                    }
                });
            }
        })
        .setup(|app| {
            db::init_db(app.handle())?;

            // Restore window state
            if let Some(window) = app.get_webview_window("main") {
                if let Ok(Some(state)) = db::load_window_state(app.handle()) {
                    let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize {
                        width: state.width,
                        height: state.height,
                    }));
                    let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition {
                        x: state.x,
                        y: state.y,
                    }));
                    let _ = window.set_always_on_top(state.pinned);
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            set_always_on_top,
            close_window,
            minimize_window,
            start_drag,
            toggle_maximize,
            select_root_directory,
            get_root_directory,
            get_root_directories,
            remove_root_directory,
            purge_files_under_root,
            purge_all_files,
            get_db_path,
            get_files_count,
            scan_files,
            scan_files_multi,
            start_watching,
            start_watching_multi,
            stop_watching,
            get_all_files,
            create_tag,
            get_all_tags,
            update_tag,
            delete_tag,
            move_tag,
            add_file_tag,
            remove_file_tag,
            get_file_tags,
            filter_files_by_tags,
            recommend_tags_by_title,
            generate_tags_llm,
            generate_image_tags_llm,
            save_window_state,
            load_window_state,
            open_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
#[tauri::command]
fn recommend_tags_by_title(
    app_handle: tauri::AppHandle,
    file_path: String,
    top_k: usize,
) -> Result<Vec<db::TagInfo>, String> {
    let tags = db::get_all_tags(&app_handle).map_err(|e| e.to_string())?;
    let path = std::path::Path::new(&file_path);
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let mut tag_names: Vec<String> = Vec::new();
    for t in &tags {
        tag_names.push(t.name.clone());
    }
    let ai_scores = ai::recommend_by_title_candle(&name, &tag_names).unwrap_or_default();
    if !ai_scores.is_empty() {
        let mut sorted: Vec<(usize, f32)> = Vec::new();
        for (i, t) in tags.iter().enumerate() {
            if let Some((_, s)) = ai_scores.iter().find(|(n, _)| n == &t.name) {
                sorted.push((i, *s));
            }
        }
        sorted.sort_by(|a, b| b.1.total_cmp(&a.1));
        let mut out = Vec::new();
        for (idx, _) in sorted.into_iter().take(top_k) {
            out.push(tags[idx].clone());
        }
        return Ok(out);
    }
    let lname = name.to_lowercase();
    let tokens: Vec<String> = lname
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    let mut scored: Vec<(db::TagInfo, i32)> = Vec::new();
    for t in tags {
        let tname = t.name.to_lowercase();
        let mut score = 0;
        if !tname.is_empty() {
            if lname.contains(&tname) {
                score += 10;
            }
            if tokens.iter().any(|w| w == &tname) {
                score += 8;
            }
            if lname.starts_with(&tname) || lname.ends_with(&tname) {
                score += 4;
            }
        }
        if score > 0 {
            scored.push((t, score));
        }
    }
    scored.sort_by(|a, b| b.1.cmp(&a.1));
    Ok(scored.into_iter().take(top_k).map(|(t, _)| t).collect())
}
