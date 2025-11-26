use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

mod db;

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
fn start_drag(window: tauri::Window) {
    let _ = window.start_dragging();
}

// Root directory commands
#[tauri::command]
async fn select_root_directory(app_handle: tauri::AppHandle) -> Result<String, String> {
    let dialog = app_handle.dialog().file();
    
    if let Some(file_path) = dialog.blocking_pick_folder() {
        if let Some(path) = file_path.as_path() {
            if let Some(path_str) = path.to_str() {
                db::set_root_directory(&app_handle, path_str.to_string())
                    .map_err(|e| e.to_string())?;
                return Ok(path_str.to_string());
            }
        }
        Err("Invalid path encoding".to_string())
    } else {
        Err("No folder selected".to_string())
    }
}

#[tauri::command]
fn get_root_directory(app_handle: tauri::AppHandle) -> Option<String> {
    db::get_root_directory(&app_handle).ok().flatten()
}

// File scanning commands
#[tauri::command]
fn scan_files(_app_handle: tauri::AppHandle, root_path: String) -> Result<Vec<db::FileListItem>, String> {
    eprintln!("🎯 [TAURI] scan_files command called with path: {}", root_path);
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
fn move_tag(app_handle: tauri::AppHandle, id: u32, new_parent_id: Option<u32>, target_position: i32) -> Result<(), String> {
    db::move_tag(&app_handle, id, new_parent_id, target_position).map_err(|e| e.to_string())
}

// File-tag relationship commands
#[tauri::command]
fn add_file_tag(app_handle: tauri::AppHandle, file_path: String, tag_id: u32) -> Result<(), String> {
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = app.get_webview_window("main").expect("no main window").set_focus();
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_) = event {
                let win = window.clone();
                std::thread::spawn(move || {
                    if let Ok(factor) = win.scale_factor() {
                        if let (Ok(pos), Ok(size)) = (win.outer_position(), win.inner_size()) {
                            let logical_pos = pos.to_logical::<f64>(factor);
                            let logical_size = size.to_logical::<f64>(factor);
                            let app_handle = win.app_handle();
                            let pinned = if let Ok(Some(state)) = db::load_window_state(app_handle) {
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
                                pinned
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
                     let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize { width: state.width, height: state.height }));
                     let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition { x: state.x, y: state.y }));
                     let _ = window.set_always_on_top(state.pinned);
                 }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            set_always_on_top,
            close_window,
            start_drag,
            select_root_directory,
            get_root_directory,
            scan_files,
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
            save_window_state,
            load_window_state
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
