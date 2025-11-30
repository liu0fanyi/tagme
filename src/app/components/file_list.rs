use leptos::prelude::*;
use leptos::task::spawn_local;
use crate::app::types::{DisplayFile, SortColumn, SortDirection, TagInfo, FileInfo, OpenFileArgs, AddFileTagArgs};
use crate::app::utils::{format_file_size, format_timestamp};
use leptos_recommender::RecommendItem;
use crate::app::api::invoke;
use crate::app::files::load_all_files;

#[component]
pub fn FileList(
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
pub fn GroupedFileList(
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
