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
    let dragged_pos = tags.iter().find(|t| t.id == dragged_id).map(|t| t.position).unwrap_or(0);
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
