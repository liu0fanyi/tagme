use leptos::prelude::*;
use wasm_bindgen::JsCast;

pub fn init_overlay(batch_running: ReadSignal<bool>, set_batch_cancel: WriteSignal<bool>) {
    Effect::new(move |_| {
        let running = batch_running.get();
        if let Some(win) = web_sys::window() {
            if let Some(doc) = win.document() {
                if let Some(body) = doc.body() {
                    let _ = body.style().set_property("overflow", if running { "hidden" } else { "" });
                }
            }
        }
        if running {
            web_sys::console::log_1(&"[Overlay] on".into());
            if let Some(win) = web_sys::window() {
                let set_cancel = set_batch_cancel;
                let on_key = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
                    if e.key() == "Escape" { set_cancel.set(true); }
                });
                let _ = win.add_event_listener_with_callback("keydown", on_key.as_ref().unchecked_ref());
                on_key.forget();
            }
        } else {
            web_sys::console::log_1(&"[Overlay] off".into());
        }
    });
}
