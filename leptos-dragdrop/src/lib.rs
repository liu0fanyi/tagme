use leptos::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Clone, Copy)]
pub struct Node {
    pub id: u32,
    pub parent_id: Option<u32>,
    pub position: i32,
}

pub fn unify_hover_target(tags: &[Node], current: Node, relative_y: f64) -> (u32, f64) {
    let mut pos = relative_y.max(0.0).min(1.0);
    let mut target = current.id;
    if pos > 0.75 {
        let mut siblings: Vec<Node> = tags.iter().copied().filter(|t| t.parent_id == current.parent_id).collect();
        siblings.sort_by_key(|t| t.position);
        if let Some(next) = siblings.into_iter().find(|t| t.position > current.position) {
            target = next.id;
            pos = 0.0;
        }
    } else if pos < 0.25 {
        pos = 0.0;
    }
    (target, pos)
}

pub fn is_descendant(tags: &[Node], ancestor: u32, descendant: u32) -> bool {
    let mut check = Some(descendant);
    while let Some(curr) = check {
        if curr == ancestor {
            return true;
        }
        check = tags.iter().find(|t| t.id == curr).and_then(|t| t.parent_id);
    }
    false
}

pub fn compute_drop_action(dragged_id: u32, target_id: u32, pos: f64, tags: &[Node]) -> Option<(Option<u32>, i32, &'static str)> {
    if dragged_id == target_id || is_descendant(tags, dragged_id, target_id) {
        return None;
    }
    let target_tag = tags.iter().find(|t| t.id == target_id).copied();
    let dragged_parent = tags.iter().find(|t| t.id == dragged_id).and_then(|t| t.parent_id);
    if let Some(tag) = target_tag {
        if pos < 0.25 {
            if tag.parent_id == dragged_parent {
                let action = "before-same-parent";
                return Some((tag.parent_id, tag.position, action));
            } else {
                let action = "before";
                return Some((tag.parent_id, tag.position, action));
            }
        } else if pos > 0.75 {
            let action = "after";
            return Some((tag.parent_id, tag.position + 1, action));
        } else {
            let action = "child";
            return Some((Some(tag.id), 0, action));
        }
    }
    Some((None, 0, "root"))
}

pub fn end_drag(set_dragging_id: WriteSignal<Option<u32>>, set_drop_target_id: WriteSignal<Option<u32>>, set_drag_just_ended: WriteSignal<bool>) {
    set_dragging_id.set(None);
    set_drop_target_id.set(None);
    set_drag_just_ended.set(true);
    if let Some(win) = web_sys::window() {
        let clear = set_drag_just_ended;
        let cb = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || {
            clear.set(false);
        });
        let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), 100);
        cb.forget();
    }
}

#[derive(Clone)]
pub struct DndSignals {
    pub dragging_id_read: ReadSignal<Option<u32>>,
    pub dragging_id_write: WriteSignal<Option<u32>>,
    pub drop_target_id_read: ReadSignal<Option<u32>>,
    pub drop_target_id_write: WriteSignal<Option<u32>>,
    pub drop_position_read: ReadSignal<f64>,
    pub drop_position_write: WriteSignal<f64>,
    pub drag_just_ended_read: ReadSignal<bool>,
    pub drag_just_ended_write: WriteSignal<bool>,
}

pub fn create_dnd_signals() -> DndSignals {
    let (dragging_id_read, dragging_id_write) = signal(None::<u32>);
    let (drop_target_id_read, drop_target_id_write) = signal(None::<u32>);
    let (drop_position_read, drop_position_write) = signal(0.5f64);
    let (drag_just_ended_read, drag_just_ended_write) = signal(false);
    DndSignals {
        dragging_id_read,
        dragging_id_write,
        drop_target_id_read,
        drop_target_id_write,
        drop_position_read,
        drop_position_write,
        drag_just_ended_read,
        drag_just_ended_write,
    }
}

pub fn make_on_mousedown(dnd: DndSignals, tag_id: u32) -> impl Fn(web_sys::MouseEvent) + Copy + 'static {
    move |ev: web_sys::MouseEvent| {
        if ev.button() == 0 {
            if let Some(target) = ev.target() {
                if target.dyn_ref::<web_sys::HtmlInputElement>().is_some() { return; }
                if target.dyn_ref::<web_sys::HtmlButtonElement>().is_some() { return; }
            }
            dnd.dragging_id_write.set(Some(tag_id));
            ev.stop_propagation();
        }
    }
}

pub fn make_on_mousemove(dnd: DndSignals, current: Node, get_nodes: impl Fn() -> Vec<Node> + Copy + 'static) -> impl Fn(web_sys::MouseEvent) + Copy + 'static {
    move |ev: web_sys::MouseEvent| {
        if dnd.dragging_id_read.get_untracked().is_some() {
            if let Some(target) = ev.current_target() {
                if let Some(element) = target.dyn_ref::<web_sys::HtmlElement>() {
                    let rect = element.get_bounding_client_rect();
                    let y = ev.client_y() as f64;
                    let top = rect.top();
                    let height = rect.height();
                    if height > 0.0 {
                        let relative_y = ((y - top) / height).max(0.0).min(1.0);
                        let nodes = get_nodes();
                        let (target_id_effective, pos_effective) = unify_hover_target(&nodes, current, relative_y);
                        dnd.drop_target_id_write.set(Some(target_id_effective));
                        dnd.drop_position_write.set(pos_effective);
                    }
                }
            }
        }
    }
}

pub fn make_label_click_guard(dnd: DndSignals) -> impl Fn(web_sys::MouseEvent) + Copy + 'static {
    move |ev: web_sys::MouseEvent| {
        if dnd.dragging_id_read.get_untracked().is_some() || dnd.drag_just_ended_read.get_untracked() {
            ev.stop_propagation();
            ev.prevent_default();
        }
    }
}

pub fn make_checkbox_change_guard(dnd: DndSignals, on_toggle: impl Fn(u32) + Copy + 'static, tag_id: u32) -> impl Fn(web_sys::Event) + Copy + 'static {
    move |ev: web_sys::Event| {
        if dnd.dragging_id_read.get_untracked().is_none() && !dnd.drag_just_ended_read.get_untracked() {
            on_toggle(tag_id);
        } else {
            ev.stop_propagation();
            ev.prevent_default();
        }
    }
}

pub fn make_checkbox_click_guard(dnd: DndSignals) -> impl Fn(web_sys::MouseEvent) + Copy + 'static {
    move |ev: web_sys::MouseEvent| {
        if dnd.dragging_id_read.get_untracked().is_some() || dnd.drag_just_ended_read.get_untracked() {
            ev.stop_propagation();
            ev.prevent_default();
        }
    }
}

pub fn bind_global_mouseup(dnd: DndSignals, get_nodes: impl Fn() -> Vec<Node> + Copy + 'static, on_drop: impl Fn(u32, Option<u32>, i32) + Copy + 'static) {
    let window = web_sys::window().unwrap();
    let on_mouseup = wasm_bindgen::closure::Closure::<dyn FnMut(_)>::new(move |_ev: web_sys::MouseEvent| {
        if let (Some(dragged_id), Some(target_id)) = (dnd.dragging_id_read.get_untracked(), dnd.drop_target_id_read.get_untracked()) {
            let pos = dnd.drop_position_read.get_untracked();
            let nodes = get_nodes();
            if let Some((new_parent_id, target_position, _action)) = compute_drop_action(dragged_id, target_id, pos, &nodes) {
                on_drop(dragged_id, new_parent_id, target_position);
            }
        }
        end_drag(dnd.dragging_id_write, dnd.drop_target_id_write, dnd.drag_just_ended_write);
    });
    let _ = window.add_event_listener_with_callback("mouseup", on_mouseup.as_ref().unchecked_ref());
    on_mouseup.forget();
}
