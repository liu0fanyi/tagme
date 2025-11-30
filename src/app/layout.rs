use leptos::prelude::*;
use wasm_bindgen::prelude::*;

pub fn init_resize_handlers(
    is_resizing_left: ReadSignal<bool>,
    set_is_resizing_left: WriteSignal<bool>,
    is_resizing_right: ReadSignal<bool>,
    set_is_resizing_right: WriteSignal<bool>,
    set_left_panel_width: WriteSignal<f64>,
    set_right_panel_width: WriteSignal<f64>,
) {
    Effect::new(move |_| {
        let window = web_sys::window().unwrap();
        let on_mousemove = Closure::<dyn FnMut(_)>::new(move |ev: web_sys::MouseEvent| {
            if is_resizing_left.get_untracked() {
                let x = ev.client_x() as f64;
                let new_width = x.max(200.0).min(600.0);
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
        let on_mouseup_resize = Closure::<dyn FnMut(_)>::new(move |_ev: web_sys::MouseEvent| {
            web_sys::console::log_1(&"Mouse up - stopping resize".into());
            set_is_resizing_left.set(false);
            set_is_resizing_right.set(false);
        });
        let _ = window.add_event_listener_with_callback("mouseup", on_mouseup_resize.as_ref().unchecked_ref());
        on_mouseup_resize.forget();
    });
}
