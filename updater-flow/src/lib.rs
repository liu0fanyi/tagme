use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct UpdateInfo {
    pub current: String,
    pub latest: Option<String>,
    pub has_update: bool,
}

pub async fn check(app_handle: AppHandle) -> Result<UpdateInfo, String> {
    let current = app_handle.package_info().version.to_string();
    let updater = app_handle.updater().map_err(|e| e.to_string())?;
    match updater.check().await.map_err(|e| e.to_string())? {
        Some(update) => Ok(UpdateInfo { current, latest: Some(update.version.clone()), has_update: true }),
        None => Ok(UpdateInfo { current, latest: None, has_update: false }),
    }
}

pub async fn install(app_handle: AppHandle) -> Result<(), String> {
    let updater = app_handle.updater().map_err(|e| e.to_string())?;
    if let Some(update) = updater.check().await.map_err(|e| e.to_string())? {
        let app = app_handle.clone();
        let bytes = update
            .download(
                |received: usize, total: Option<u64>| {
                    let _ = app.emit("update-download-progress", serde_json::json!({"received": received, "total": total}));
                },
                || {},
            )
            .await
            .map_err(|e| e.to_string())?;
        let _ = app_handle.emit("update-download-complete", ());
        update.install(bytes).map_err(|e| e.to_string())?;
    }
    Ok(())
}

