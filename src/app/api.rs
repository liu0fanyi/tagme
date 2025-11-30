use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    pub async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct OpenFileArgs { path: String }

pub async fn open_file(path: String) {
    let args = OpenFileArgs { path };
    let _ = invoke("open_file", serde_wasm_bindgen::to_value(&args).unwrap()).await;
}
