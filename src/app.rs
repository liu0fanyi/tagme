use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
pub mod api;
pub mod components;
pub mod drag_drop;
pub mod files;
pub mod resizing;
pub mod types;
mod update;
pub mod utils;

use crate::app::api::invoke;
use crate::app::components::file_list::*;
use crate::app::components::tag_tree::*;
use crate::app::drag_drop::*;
use crate::app::files::*;
use crate::app::resizing::*;
use crate::app::types::*;
use crate::app::utils::*;
use leptos_recommender::RecommendItem;

#[component]
pub fn App() -> impl IntoView {
    let (root_directories, set_root_directories) = signal(Vec::<String>::new());
    let (scanned_files, set_scanned_files) = signal(Vec::<FileListItem>::new());
    let (all_files, set_all_files) = signal(Vec::<FileInfo>::new());
    let (all_tags, set_all_tags) = signal(Vec::<TagInfo>::new());
    let (selected_tag_ids, set_selected_tag_ids) = signal(Vec::<u32>::new());
    let (use_and_logic, set_use_and_logic) = signal(true);
    let (displayed_files, set_displayed_files) = signal(Vec::<FileInfo>::new());
    let (file_tags_map, set_file_tags_map) =
        signal(std::collections::HashMap::<u32, Vec<TagInfo>>::new());
    let (selected_file_paths, set_selected_file_paths) = signal(Vec::<String>::new());
    let (last_selected_file_path, set_last_selected_file_path) = signal(None::<String>);
    let (file_recommended_tags_map, set_file_recommended_tags_map) =
        signal(std::collections::HashMap::<u32, Vec<TagInfo>>::new());
    let (file_recommended_info_map, set_file_recommended_info_map) =
        signal(std::collections::HashMap::<String, Vec<RecommendItem>>::new());
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
                    let _ = body
                        .style()
                        .set_property("overflow", if running { "hidden" } else { "" });
                }
            }
        }
        if running {
            web_sys::console::log_1(&"[Overlay] on".into());
            if let Some(win) = web_sys::window() {
                let set_cancel = set_batch_cancel;
                let on_key =
                    wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(
                        move |e: web_sys::KeyboardEvent| {
                            if e.key() == "Escape" {
                                set_cancel.set(true);
                            }
                        },
                    );
                let _ = win
                    .add_event_listener_with_callback("keydown", on_key.as_ref().unchecked_ref());
                on_key.forget();
            }
        } else {
            web_sys::console::log_1(&"[Overlay] off".into());
        }
    });
    let recommend_all = move |_| {
        if batch_running.get() {
            return;
        }
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
                if cancel_sig.get_untracked() {
                    break;
                }
                let path = f.path.clone();
                let label_names: Vec<String> = tags.iter().map(|t| t.name.clone()).collect();
                let tk = core::cmp::min(label_names.len(), 8);
                let list_ext = leptos_recommender::generate_for_file(
                    path.clone(),
                    label_names.clone(),
                    tk,
                    0.6,
                    Some(String::from("https://api.siliconflow.cn/v1")),
                    None,
                )
                .await;
                if !list_ext.is_empty() {
                    let list: Vec<RecommendItem> = list_ext
                        .into_iter()
                        .map(|ri| RecommendItem {
                            name: ri.name,
                            score: ri.score,
                            source: ri.source,
                        })
                        .collect();
                    info_map.insert(path.clone(), list.clone());
                    let mut out: Vec<TagInfo> = Vec::new();
                    for item in list {
                        if let Some(t) = tags.iter().find(|x| x.name == item.name) {
                            out.push(t.clone());
                        }
                    }
                    tag_map.insert(f.id, out);
                }
                set_prog.set(i + 1);
                if i % 5 == 4 {
                    set_map.set(tag_map.clone());
                    set_info.set(info_map.clone());
                }
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
    let (update_loading, set_update_loading) = signal(false);
    let (update_downloading, set_update_downloading) = signal(false);
    let (update_received, set_update_received) = signal(0usize);
    let (update_total, set_update_total) = signal(None::<u64>);
    // 检查更新的错误信息（超时或失败时设置，用于弹窗提示）
    let (update_error, set_update_error) = signal(None::<String>);
    // 下次重试的秒数（例如 600 表示 10 分钟后重试，用于弹窗展示）
    let (update_retry_in, set_update_retry_in) = signal(None::<u32>);
    leptos_updater::init_update_system(leptos_updater::UpdaterArgs {
        set_show_update_modal,
        show_update_modal,
        update_current,
        set_update_current,
        update_latest,
        set_update_latest,
        update_has,
        set_update_has,
        update_error,
        set_update_error,
        update_retry_in,
        set_update_retry_in,
        update_downloading,
        set_update_downloading,
        update_received,
        set_update_received,
        update_total,
        set_update_total,
    });

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
            let name = path_obj
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let extension = path_obj
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

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
                    let name = path_obj
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let extension = path_obj
                        .extension()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();

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
            set_sort_direction.update(|d| {
                *d = match d {
                    SortDirection::Asc => SortDirection::Desc,
                    SortDirection::Desc => SortDirection::Asc,
                }
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
    let (drag_just_ended, set_drag_just_ended) = signal(false);
    let dnd = leptos_dragdrop::DndSignals {
        dragging_id_read: dragging_tag_id,
        dragging_id_write: set_dragging_tag_id,
        drop_target_id_read: drop_target_tag_id,
        drop_target_id_write: set_drop_target_tag_id,
        drop_position_read: drop_position,
        drop_position_write: set_drop_position,
        drag_just_ended_read: drag_just_ended,
        drag_just_ended_write: set_drag_just_ended,
    };
    let (reload_tags_trigger, set_reload_tags_trigger) = signal(0u32);
    let (last_click_time, set_last_click_time) = signal(0.0);
    let (is_maximized, set_is_maximized) = signal(false);

    // Global mouse up handler for drag and drop
    setup_drag_drop(
        dragging_tag_id,
        set_dragging_tag_id,
        drop_target_tag_id,
        set_drop_target_tag_id,
        drop_position,
        set_drop_position,
        set_drag_just_ended,
        all_tags,
        set_reload_tags_trigger,
    );

    // Global mouse handlers for panel resizing
    setup_resizing(
        is_resizing_left,
        set_is_resizing_left,
        is_resizing_right,
        set_is_resizing_right,
        set_left_panel_width,
        set_right_panel_width,
    );

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
            let roots: Result<Vec<String>, _> =
                serde_wasm_bindgen::from_value(invoke("get_root_directories", JsValue::NULL).await);
            match roots {
                Ok(list) => {
                    if list.is_empty() {
                        let root: Option<String> = serde_wasm_bindgen::from_value(
                            invoke("get_root_directory", JsValue::NULL).await,
                        )
                        .unwrap_or(None);
                        if let Some(p) = root {
                            set_root_directories.set(vec![p]);
                        }
                    } else {
                        set_root_directories.set(list);
                    }
                }
                Err(_) => {
                    let root: Option<String> = serde_wasm_bindgen::from_value(
                        invoke("get_root_directory", JsValue::NULL).await,
                    )
                    .unwrap_or(None);
                    if let Some(p) = root {
                        set_root_directories.set(vec![p]);
                    }
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
                    struct StartWatchingMultiArgs {
                        root_paths: Vec<String>,
                    }
                    let args = StartWatchingMultiArgs {
                        root_paths: list.clone(),
                    };
                    let _ = invoke(
                        "start_watching_multi",
                        serde_wasm_bindgen::to_value(&args).unwrap(),
                    )
                    .await;
                });
            }

            let list2 = root_directories.get_untracked();
            if !list2.is_empty() {
                spawn_local(async move {
                    #[derive(Serialize)]
                    #[serde(rename_all = "camelCase")]
                    struct ScanFilesMultiArgs {
                        root_paths: Vec<String>,
                    }
                    let args = ScanFilesMultiArgs {
                        root_paths: list2.clone(),
                    };
                    if let Ok(files) = serde_wasm_bindgen::from_value::<Vec<FileListItem>>(
                        invoke(
                            "scan_files_multi",
                            serde_wasm_bindgen::to_value(&args).unwrap(),
                        )
                        .await,
                    ) {
                        set_scanned_files.set(files);
                        load_all_files(set_all_files, set_displayed_files, set_file_tags_map).await;
                    }
                });
            }

            // Setup file system change listener
            let setup_listener = js_sys::Function::new_no_args(
                r#"
                console.log('🔧 [FRONTEND] Setting up Tauri event listener...');
                if (window.__TAURI__ && window.__TAURI__.event) {
                    if (window.__TAGME_UPDATE_LISTENER_SET) { console.log('ℹ️ update listeners already set'); } else { window.__TAGME_UPDATE_LISTENER_SET = true; }
                    window.__TAURI__.event.listen('file-system-change', () => {
                        console.log('📬 [FRONTEND] File change detected by Tauri');
                        window.dispatchEvent(new CustomEvent('tauri-fs-change'));
                        console.log('✅ [FRONTEND] Custom event dispatched');
                    });
                    window.__TAURI__.event.listen('update-download-progress', (evt) => {
                        const payload = evt && evt.payload ? evt.payload : {};
                        window.dispatchEvent(new CustomEvent('tauri-update-progress', { detail: payload }));
                    });
                    window.__TAURI__.event.listen('update-download-complete', () => {
                        window.dispatchEvent(new CustomEvent('tauri-update-complete'));
                    });
                    console.log('✅ [FRONTEND] Tauri event listener registered');
                } else {
                    console.error('❌ [FRONTEND] Tauri event API not available');
                }
            "#,
            );
            let _ = setup_listener.call0(&JsValue::NULL);
        });
    });

    // Listen for custom file change events and trigger scan
    Effect::new(move |_| {
        let window = web_sys::window().expect("no window");
        web_sys::console::log_1(
            &"🎧 [FRONTEND] Registering custom event listener for 'tauri-fs-change'".into(),
        );
        let flag = js_sys::Reflect::get(&window, &JsValue::from_str("__TAGME_FS_LISTENER_SET"))
            .ok()
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !flag {
            let closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
                web_sys::console::log_1(
                    &"📥 [FRONTEND] Custom event received, refreshing file list...".into(),
                );
                let list = root_directories.get_untracked();
                if !list.is_empty() {
                    set_scanning.set(true);
                    spawn_local(async move {
                        #[derive(Serialize)]
                        #[serde(rename_all = "camelCase")]
                        struct ScanFilesMultiArgs {
                            root_paths: Vec<String>,
                        }
                        let args = ScanFilesMultiArgs {
                            root_paths: list.clone(),
                        };
                        if let Ok(files) = serde_wasm_bindgen::from_value::<Vec<FileListItem>>(
                            invoke(
                                "scan_files_multi",
                                serde_wasm_bindgen::to_value(&args).unwrap(),
                            )
                            .await,
                        ) {
                            set_scanned_files.set(files);
                            load_all_files(set_all_files, set_displayed_files, set_file_tags_map)
                                .await;
                        }
                        set_scanning.set(false);
                    });
                }
            }) as Box<dyn FnMut(_)>);
            let _ = window.add_event_listener_with_callback(
                "tauri-fs-change",
                closure.as_ref().unchecked_ref(),
            );
            let _ = js_sys::Reflect::set(
                &window,
                &JsValue::from_str("__TAGME_FS_LISTENER_SET"),
                &JsValue::from_bool(true),
            );
            web_sys::console::log_1(&"✅ [FRONTEND] Custom event listener registered".into());
            closure.forget();
        }
    });

    Effect::new(move |_| {
        let window = web_sys::window().expect("no window");
        let flag = js_sys::Reflect::get(
            &window,
            &JsValue::from_str("__TAGME_UPDATE_PROGRESS_LISTENER_SET"),
        )
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
        if !flag {
            let closure = Closure::wrap(Box::new(move |ev: web_sys::Event| {
                if let Some(ce) = ev.dyn_ref::<web_sys::CustomEvent>() {
                    let detail = ce.detail();
                    let rec = js_sys::Reflect::get(&detail, &JsValue::from_str("received"))
                        .ok()
                        .and_then(|v| v.as_f64())
                        .map(|x| x as usize)
                        .unwrap_or(0usize);
                    let tot = js_sys::Reflect::get(&detail, &JsValue::from_str("total"))
                        .ok()
                        .and_then(|v| {
                            if v.is_null() || v.is_undefined() {
                                None
                            } else {
                                v.as_f64().map(|x| x as u64)
                            }
                        });
                    set_update_received.set(rec);
                    set_update_total.set(tot);
                    set_update_downloading.set(true);
                }
            }) as Box<dyn FnMut(_)>);
            let _ = window.add_event_listener_with_callback(
                "tauri-update-progress",
                closure.as_ref().unchecked_ref(),
            );
            let _ = js_sys::Reflect::set(
                &window,
                &JsValue::from_str("__TAGME_UPDATE_PROGRESS_LISTENER_SET"),
                &JsValue::from_bool(true),
            );
            closure.forget();
        }
    });

    Effect::new(move |_| {
        let window = web_sys::window().expect("no window");
        let flag = js_sys::Reflect::get(
            &window,
            &JsValue::from_str("__TAGME_UPDATE_COMPLETE_LISTENER_SET"),
        )
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
        if !flag {
            let closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
                set_update_downloading.set(false);
            }) as Box<dyn FnMut(_)>);
            let _ = window.add_event_listener_with_callback(
                "tauri-update-complete",
                closure.as_ref().unchecked_ref(),
            );
            let _ = js_sys::Reflect::set(
                &window,
                &JsValue::from_str("__TAGME_UPDATE_COMPLETE_LISTENER_SET"),
                &JsValue::from_bool(true),
            );
            closure.forget();
        }
    });

    Effect::new(move || {
        spawn_local(async move {
            // 启动时进行一次后台检查，加入 8 秒超时控制，避免网络不佳时卡住体验
            let window = web_sys::window().expect("no window");
            // done 用于在超时回调中判断异步检查是否已完成
            let done = std::rc::Rc::new(std::cell::Cell::new(false));
            let done2 = done.clone();
            // 8 秒超时：若检查仍未完成，则设置错误与重试信息（10 分钟后重试）
            let timeout_cb = Closure::wrap(Box::new(move || {
                if !done2.get() {
                    set_update_error.set(Some(format!("检查更新超时，将在{}分钟后重试", 10)));
                    set_update_retry_in.set(Some(600));
                }
            }) as Box<dyn FnMut()>);
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                timeout_cb.as_ref().unchecked_ref(),
                8000,
            );
            timeout_cb.forget();

            // 实际检查更新：成功则更新版本信息；失败则提示并设置重试
            let val = invoke("updater_check", JsValue::NULL).await;
            match serde_wasm_bindgen::from_value::<UpdateInfo>(val.clone()) {
                Ok(info) => {
                    // 检查成功，清理错误提示与重试信息，并更新版本状态
                    done.set(true);
                    set_update_error.set(None);
                    set_update_retry_in.set(None);
                    set_update_current.set(info.current);
                    set_update_latest.set(info.latest.unwrap_or_default());
                    set_update_has.set(info.has_update);
                }
                Err(_) => {
                    // 检查失败，提示失败并设置 10 分钟后重试
                    done.set(true);
                    set_update_error.set(Some(format!("检查更新失败，将在{}分钟后重试", 10)));
                    set_update_retry_in.set(Some(600));
                }
            }
        });
    });

    Effect::new(move |_| {
        let window = web_sys::window().expect("no window");
        let flag = js_sys::Reflect::get(
            &window,
            &JsValue::from_str("__TAGME_AUTO_UPDATE_INTERVAL_SET"),
        )
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
        if !flag {
            let set_c = set_update_current;
            let set_l = set_update_latest;
            let set_h = set_update_has;
            // 后台定时检查也维护错误与重试提示（无加载遮挡）
            let set_err = set_update_error;
            let set_retry = set_update_retry_in;
            let cb = Closure::wrap(Box::new(move || {
                let set_c2 = set_c;
                let set_l2 = set_l;
                let set_h2 = set_h;
                let set_err2 = set_err;
                let set_retry2 = set_retry;
                spawn_local(async move {
                    let window = web_sys::window().expect("no window");
                    // 8 秒超时控制，避免后台任务长时间未返回
                    let done = std::rc::Rc::new(std::cell::Cell::new(false));
                    let done2 = done.clone();
                    let timeout_cb = Closure::wrap(Box::new(move || {
                        if !done2.get() {
                            set_err2.set(Some(format!("检查更新超时，将在{}分钟后重试", 10)));
                            set_retry2.set(Some(600));
                        }
                    }) as Box<dyn FnMut()>);
                    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                        timeout_cb.as_ref().unchecked_ref(),
                        8000,
                    );
                    timeout_cb.forget();

                    // 定时检查更新逻辑
                    let val = invoke("updater_check", JsValue::NULL).await;
                    match serde_wasm_bindgen::from_value::<UpdateInfo>(val.clone()) {
                        Ok(info) => {
                            // 检查成功，清理错误与重试信息，并刷新版本状态
                            done.set(true);
                            set_err2.set(None);
                            set_retry2.set(None);
                            set_c2.set(info.current);
                            set_l2.set(info.latest.unwrap_or_default());
                            set_h2.set(info.has_update);
                        }
                        Err(_) => {
                            // 检查失败，设置提示与 10 分钟后重试
                            done.set(true);
                            set_err2.set(Some(format!("检查更新失败，将在{}分钟后重试", 10)));
                            set_retry2.set(Some(600));
                        }
                    }
                });
            }) as Box<dyn FnMut()>);
            let _ = window.set_interval_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                600000,
            );
            let _ = js_sys::Reflect::set(
                &window,
                &JsValue::from_str("__TAGME_AUTO_UPDATE_INTERVAL_SET"),
                &JsValue::from_bool(true),
            );
            cb.forget();
        }
    });

    let select_directory = move |_| {
        handle_select_directory(
            root_directories,
            set_root_directories,
            set_scanning,
            set_scanned_files,
            set_all_files,
            set_displayed_files,
            set_file_tags_map,
            active_root_filter,
            set_active_root_filter,
        );
    };

    let scan_directory = move |_| {
        handle_scan_directory(
            root_directories,
            set_scanning,
            set_scanned_files,
            set_all_files,
            set_displayed_files,
            set_file_tags_map,
        );
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
        web_sys::console::log_1(
            &format!(
                "toggle_tag_selection start, tag_id={}, before={:?}",
                tag_id, current
            )
            .into(),
        );
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
        web_sys::console::log_1(
            &format!(
                "should_select={}, subtree_ids={:?}",
                should_select, subtree_ids
            )
            .into(),
        );
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
        web_sys::console::log_1(
            &format!(
                "filter_files with {} tags, use_and={}, force_or={}",
                current.len(),
                logic,
                force_or
            )
            .into(),
        );
        filter_files(current, logic, set_displayed_files, all_files.get());
    };

    let toggle_and_or = move |_| {
        let new_logic = !use_and_logic.get();
        set_use_and_logic.set(new_logic);
        filter_files(
            selected_tag_ids.get(),
            new_logic,
            set_displayed_files,
            all_files.get(),
        );
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
                let args = CreateTagArgs {
                    name,
                    parent_id: parent,
                    color: None,
                };
                let _ = invoke("create_tag", serde_wasm_bindgen::to_value(&args).unwrap()).await;
                load_tags(set_all_tags).await;
                set_show_add_tag_dialog.set(false);
                set_new_tag_name.set(String::new());
                set_new_tag_parent.set(None);
            });
        }
    };

    provide_context(dnd.clone());
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
                    {leptos_updater::UpdateHeaderButton(leptos_updater::UpdateHeaderButtonProps { args: leptos_updater::UpdaterArgs {
                        set_show_update_modal,
                        show_update_modal,
                        update_current,
                        set_update_current,
                        update_latest,
                        set_update_latest,
                        update_has,
                        set_update_has,
                        update_error,
                        set_update_error,
                        update_retry_in,
                        set_update_retry_in,
                        update_downloading,
                        set_update_downloading,
                        update_received,
                        set_update_received,
                        update_total,
                        set_update_total,
                    }})}
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
                                                style="padding:2px 6px; border-radius:4px; display:inline-flex; align-items:center; gap:6px; cursor:pointer;"
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
                        dnd=dnd.clone()
                        drag_just_ended=drag_just_ended
                        set_drag_just_ended=set_drag_just_ended
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
                                            let list_ext = leptos_recommender::generate_for_file(path.clone(), label_names.clone(), tk, 0.6, Some(String::from("https://api.siliconflow.cn/v1")), None).await;
                                            if !list_ext.is_empty() {
                                                let list: Vec<RecommendItem> = list_ext.into_iter().map(|ri| RecommendItem { name: ri.name, score: ri.score, source: ri.source }).collect();
                                                let mut map = file_recommended_info_map.get_untracked();
                                                map.insert(path.clone(), list);
                                                set_info.set(map);
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

            {leptos_updater::UpdateModal(leptos_updater::UpdateModalProps { args: leptos_updater::UpdaterArgs {
                set_show_update_modal,
                show_update_modal,
                update_current,
                set_update_current,
                update_latest,
                set_update_latest,
                update_has,
                set_update_has,
                update_error,
                set_update_error,
                update_retry_in,
                set_update_retry_in,
                update_downloading,
                set_update_downloading,
                update_received,
                set_update_received,
                update_total,
                set_update_total,
            }})}

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

            {move || (update_loading.get() || update_downloading.get()).then(|| view! {
                <div class="overlay-blocker" style="position:fixed;top:0;left:0;right:0;bottom:0;background:rgba(0,0,0,0.45);z-index:3000;display:flex;align-items:center;justify-content:center;">
                    <div class="overlay-card" style="background: var(--bg-secondary); border: 1px solid var(--border-color); border-radius: 12px; padding: 16px; min-width: 320px;">
                        {move || {
                            let is_checking = update_loading.get();
                            let total = update_total.get();
                            let received = update_received.get();
                            let pct = if !is_checking { if let Some(t) = total { if t>0 { (received as f64 / t as f64 * 100.0) as i32 } else { 0 } } else { -1 } } else { -1 };
                            let title = if is_checking { "Checking for updates...".to_string() } else { if pct >= 0 { format!("Downloading... {}%", pct) } else { "Downloading...".to_string() } };
                            let width_text = if is_checking { "width: 25%".to_string() } else { if pct >= 0 { format!("width: {}%", pct) } else { "width: 25%".to_string() } };
                            view! { <div>
                                <div>{title}</div>
                                <div class="progress-bar" style="width: 100%; height: 8px; background: var(--bg-primary); border-radius: 4px; margin-top: 8px;"><div class="progress-fill" style=move || width_text.clone()></div></div>
                            </div> }
                        }}
                    </div>
                </div>
            })}
        </div>
    }
}

#[derive(Clone, Debug, Deserialize)]
struct UpdateInfo {
    current: String,
    latest: Option<String>,
    has_update: bool,
}
