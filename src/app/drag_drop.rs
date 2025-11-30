use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use crate::app::types::*;
use crate::app::api::invoke;

pub fn setup_drag_drop(
    dragging_tag_id: ReadSignal<Option<u32>>,
    set_dragging_tag_id: WriteSignal<Option<u32>>,
    drop_target_tag_id: ReadSignal<Option<u32>>,
    set_drop_target_tag_id: WriteSignal<Option<u32>>,
    drop_position: ReadSignal<f64>,
    set_drop_position: WriteSignal<f64>,
    set_drag_just_ended: WriteSignal<bool>,
    all_tags: ReadSignal<Vec<TagInfo>>,
    set_reload_tags_trigger: WriteSignal<u32>,
) {
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

                        let nodes: Vec<leptos_dragdrop::Node> = all_tags
                            .get_untracked()
                            .iter()
                            .map(|t| leptos_dragdrop::Node { id: t.id, parent_id: t.parent_id, position: t.position })
                            .collect();
                        if let Some((new_parent_id, target_position, action)) = leptos_dragdrop::compute_drop_action(dragged_id, target_id, pos, &nodes) {
                            web_sys::console::log_1(&format!("🎯 Action: {}, Parent: {:?}, Position: {}", action, new_parent_id, target_position).into());
                            spawn_local(async move {
                                let args = MoveTagArgs { id: dragged_id, new_parent_id, target_position };
                                let _ = invoke("move_tag", serde_wasm_bindgen::to_value(&args).unwrap()).await;
                                set_reload_tags_trigger.update(|v| *v += 1);
                            });
                        } else {
                            web_sys::console::log_1(&"⚠️ Cannot drop - invalid target".into());
                        }
                    }
                }
                
                leptos_dragdrop::end_drag(set_dragging_tag_id, set_drop_target_tag_id, set_drag_just_ended);
            }
        });
        
        let _ = window.add_event_listener_with_callback("mouseup", on_mouseup.as_ref().unchecked_ref());
        on_mouseup.forget();
    });
}
