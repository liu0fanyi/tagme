use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::app::types::{TagInfo, FileInfo, DeleteTagArgs};
use crate::app::files::filter_files;
use crate::app::api::invoke;

#[component]
pub fn TagTree(
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
    drag_just_ended: ReadSignal<bool>,
    set_drag_just_ended: WriteSignal<bool>,
    dnd: leptos_dragdrop::DndSignals,
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
                        drag_just_ended=drag_just_ended
                        set_drag_just_ended=set_drag_just_ended
                        />
                    }
                }
            />
        </div>
    }
}

#[component]
pub fn TagNode(
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
    drag_just_ended: ReadSignal<bool>,
    set_drag_just_ended: WriteSignal<bool>,
) -> AnyView {
    let dnd = expect_context::<leptos_dragdrop::DndSignals>();
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
    let on_mousedown = leptos_dragdrop::make_on_mousedown(dnd.clone(), tag_id);

    // Mouse enter - track potential drop target
    let update_position = move |ev: &web_sys::MouseEvent| {
        if dragging_tag_id.get_untracked().is_some() {
            // Calculate relative position (0.0 = top, 1.0 = bottom)
            if let Some(target) = ev.current_target() {
                if let Some(element) = target.dyn_ref::<web_sys::HtmlElement>() {
                    let rect = element.get_bounding_client_rect();
                    let y = ev.client_y() as f64;
                    let top = rect.top();
                    let height = rect.height();
                    
                    if height > 0.0 {
                        let relative_y = ((y - top) / height).max(0.0).min(1.0);
                        let nodes: Vec<leptos_dragdrop::Node> = all_tags
                            .get_untracked()
                            .iter()
                            .map(|t| leptos_dragdrop::Node { id: t.id, parent_id: t.parent_id, position: t.position })
                            .collect();
                        let current = leptos_dragdrop::Node { id: tag_id, parent_id: tag.parent_id, position: tag.position };
                        let (target_id_effective, pos_effective) = leptos_dragdrop::unify_hover_target(&nodes, current, relative_y);
                        set_drop_target_tag_id.set(Some(target_id_effective));
                        set_drop_position.set(pos_effective);
                        web_sys::console::log_1(&format!("📍 Tag {} -> target {} position: {:.2}", tag_id, target_id_effective, pos_effective).into());
                    }
                }
            }
        }
    };

    let get_nodes = move || {
        all_tags.get_untracked().iter().map(|t| leptos_dragdrop::Node { id: t.id, parent_id: t.parent_id, position: t.position }).collect::<Vec<_>>()
    };
    let current_node = leptos_dragdrop::Node { id: tag_id, parent_id: tag.parent_id, position: tag.position };
    let on_mouseenter = leptos_dragdrop::make_on_mousemove(dnd.clone(), current_node, get_nodes);
    let on_mousemove = leptos_dragdrop::make_on_mousemove(dnd.clone(), current_node, get_nodes);

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
                on:click=leptos_dragdrop::make_label_click_guard(dnd.clone())
            >
                <input
                    type="checkbox"
                    prop:checked=is_selected
                    on:change=leptos_dragdrop::make_checkbox_change_guard(dnd.clone(), on_toggle, tag_id)
                    on:click=leptos_dragdrop::make_checkbox_click_guard(dnd.clone())
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
                                drag_just_ended=drag_just_ended
                                set_drag_just_ended=set_drag_just_ended
                                />
                            }
                        }
                    />
                </div>
            })}
        </div>
    }.into_any()
}
