use leptos::prelude::*;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[derive(Clone)]
pub struct UpdaterArgs {
    pub set_update_current: WriteSignal<String>,
    pub set_update_latest: WriteSignal<String>,
    pub set_update_has: WriteSignal<bool>,
    pub set_update_error: WriteSignal<Option<String>>, 
    pub set_update_retry_in: WriteSignal<Option<u32>>, 
    pub set_update_downloading: WriteSignal<bool>,
    pub set_update_received: WriteSignal<usize>,
    pub set_update_total: WriteSignal<Option<u64>>, 
}

#[derive(serde::Deserialize, Clone)]
struct UpdateInfo { current: String, latest: Option<String>, has_update: bool }

pub fn init_update_system(args: UpdaterArgs) {
    let a0 = args.clone();
    let a1 = args.clone();
    let a2 = args.clone();
    let a3 = args.clone();
    Effect::new(move || {
        let args = a0.clone();
        spawn_local(async move {
            let window = web_sys::window().expect("no window");
            let done = std::rc::Rc::new(std::cell::Cell::new(false));
            let done2 = done.clone();
            let timeout_cb = Closure::wrap(Box::new(move || {
                if !done2.get() {
                    args.set_update_error.set(Some(format!("检查更新超时，将在{}分钟后重试", 10)));
                    args.set_update_retry_in.set(Some(600));
                }
            }) as Box<dyn FnMut()>);
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(timeout_cb.as_ref().unchecked_ref(), 8000);
            timeout_cb.forget();

            let val = invoke("updater_check", JsValue::NULL).await;
            match serde_wasm_bindgen::from_value::<UpdateInfo>(val.clone()) {
                Ok(info) => {
                    done.set(true);
                    args.set_update_error.set(None);
                    args.set_update_retry_in.set(None);
                    args.set_update_current.set(info.current);
                    args.set_update_latest.set(info.latest.unwrap_or_default());
                    args.set_update_has.set(info.has_update);
                },
                Err(_) => {
                    done.set(true);
                    args.set_update_error.set(Some(format!("检查更新失败，将在{}分钟后重试", 10)));
                    args.set_update_retry_in.set(Some(600));
                }
            }
        });
    });

    Effect::new(move |_| {
        let window = web_sys::window().expect("no window");
        let flag = js_sys::Reflect::get(&window, &JsValue::from_str("__TAGME_AUTO_UPDATE_INTERVAL_SET")).ok().and_then(|v| v.as_bool()).unwrap_or(false);
        if !flag {
            let args = a1.clone();
            let cb = Closure::wrap(Box::new(move || {
                let args2 = args.clone();
                spawn_local(async move {
                    let window = web_sys::window().expect("no window");
                    let done = std::rc::Rc::new(std::cell::Cell::new(false));
                    let done2 = done.clone();
                    let timeout_cb = Closure::wrap(Box::new(move || {
                        if !done2.get() {
                            args2.set_update_error.set(Some(format!("检查更新超时，将在{}分钟后重试", 10)));
                            args2.set_update_retry_in.set(Some(600));
                        }
                    }) as Box<dyn FnMut()>);
                    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(timeout_cb.as_ref().unchecked_ref(), 8000);
                    timeout_cb.forget();

                    let val = invoke("updater_check", JsValue::NULL).await;
                    match serde_wasm_bindgen::from_value::<UpdateInfo>(val.clone()) {
                        Ok(info) => {
                            done.set(true);
                            args2.set_update_error.set(None);
                            args2.set_update_retry_in.set(None);
                            args2.set_update_current.set(info.current);
                            args2.set_update_latest.set(info.latest.unwrap_or_default());
                            args2.set_update_has.set(info.has_update);
                        },
                        Err(_) => {
                            done.set(true);
                            args2.set_update_error.set(Some(format!("检查更新失败，将在{}分钟后重试", 10)));
                            args2.set_update_retry_in.set(Some(600));
                        }
                    }
                });
            }) as Box<dyn FnMut()>);
            let _ = window.set_interval_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), 600000);
            let _ = js_sys::Reflect::set(&window, &JsValue::from_str("__TAGME_AUTO_UPDATE_INTERVAL_SET"), &JsValue::from_bool(true));
            cb.forget();
        }
    });

    Effect::new(move |_| {
        let window = web_sys::window().expect("no window");
        let flag = js_sys::Reflect::get(&window, &JsValue::from_str("__TAGME_UPDATE_PROGRESS_LISTENER_SET")).ok().and_then(|v| v.as_bool()).unwrap_or(false);
        if !flag {
            let set_received = a2.set_update_received;
            let set_total = a2.set_update_total;
            let set_downloading = a2.set_update_downloading;
            let closure = Closure::wrap(Box::new(move |ev: web_sys::Event| {
                if let Some(ce) = ev.dyn_ref::<web_sys::CustomEvent>() {
                    let detail = ce.detail();
                    let rec = js_sys::Reflect::get(&detail, &JsValue::from_str("received")).ok().and_then(|v| v.as_f64()).map(|x| x as usize).unwrap_or(0usize);
                    let tot = js_sys::Reflect::get(&detail, &JsValue::from_str("total")).ok().and_then(|v| if v.is_null() || v.is_undefined() { None } else { v.as_f64().map(|x| x as u64) });
                    set_received.set(rec);
                    set_total.set(tot);
                    set_downloading.set(true);
                }
            }) as Box<dyn FnMut(_)>);
            let _ = window.add_event_listener_with_callback("tauri-update-progress", closure.as_ref().unchecked_ref());
            let _ = js_sys::Reflect::set(&window, &JsValue::from_str("__TAGME_UPDATE_PROGRESS_LISTENER_SET"), &JsValue::from_bool(true));
            closure.forget();
        }
    });

    Effect::new(move |_| {
        let window = web_sys::window().expect("no window");
        let flag = js_sys::Reflect::get(&window, &JsValue::from_str("__TAGME_UPDATE_COMPLETE_LISTENER_SET")).ok().and_then(|v| v.as_bool()).unwrap_or(false);
        if !flag {
            let set_downloading = a3.set_update_downloading;
            let closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
                set_downloading.set(false);
            }) as Box<dyn FnMut(_)>);
            let _ = window.add_event_listener_with_callback("tauri-update-complete", closure.as_ref().unchecked_ref());
            let _ = js_sys::Reflect::set(&window, &JsValue::from_str("__TAGME_UPDATE_COMPLETE_LISTENER_SET"), &JsValue::from_bool(true));
            closure.forget();
        }
    });
}
