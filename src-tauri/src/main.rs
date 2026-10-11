#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use resonance_engine::{Engine, Preferences, Profile};
use serde_json::{json, Value};
use std::path::PathBuf;
use tauri_plugin_updater::UpdaterExt;

#[tauri::command]
async fn engine_command(action: String, payload: Value) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || run(&action, payload))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{e:#}"))
}
fn run(action: &str, p: Value) -> anyhow::Result<Value> {
    let e = Engine::default_paths();
    let out = match action {
        "status" => serde_json::to_value(e.status()?)?,
        "catalog" => serde_json::to_value(e.catalog()?)?,
        "font_css" => serde_json::to_value(e.font_css()?)?,
        "import_font" => {
            serde_json::to_value(e.import_sf_font(&PathBuf::from(p.as_str().unwrap_or_default()))?)?
        }
        "store_cached" => serde_json::to_value(e.store_cached())?,
        "store_refresh" => serde_json::to_value(e.store_refresh()?)?,
        "store_prepare" => serde_json::to_value(e.store_prepare(p.as_str().unwrap_or_default())?)?,
        "store_install" => serde_json::to_value(e.store_install(
            p["id"].as_str().unwrap_or_default(),
            p["sha256"].as_str().unwrap_or_default(),
        )?)?,
        "profiles" => serde_json::to_value(e.profiles()?)?,
        "installed" => json!(e.installed()?),
        "preferences" => json!(e.preferences()),
        "apply" | "repair" | "safe" => {
            json!(e.apply(&serde_json::from_value::<Profile>(p)?, action == "safe")?)
        }
        "restore" => json!(e.restore()?),
        "backup" => json!(e.backup()?),
        "recover" => json!(e.recover()?),
        "launch" => {
            e.launch()?;
            json!("Spotify launched")
        }
        "save_profile" => {
            e.save_profile(&serde_json::from_value(p)?)?;
            json!("Profile saved")
        }
        "delete_profile" => {
            e.delete_profile(p.as_str().unwrap_or_default())?;
            json!("Profile removed")
        }
        "install" => {
            e.install(p.as_str().unwrap_or_default())?;
            json!("Package installed")
        }
        "uninstall" => {
            e.uninstall(p.as_str().unwrap_or_default())?;
            json!("Package removed; apply your pending changes to unload it")
        }
        "save_preferences" => {
            e.save_preferences(&serde_json::from_value::<Preferences>(p)?)?;
            json!("Preferences saved")
        }
        "validate_css" => {
            resonance_engine::validate_css(p.as_str().unwrap_or_default())?;
            json!("CSS is valid")
        }
        "export_profile" => {
            let profile: Profile = serde_json::from_value(p["profile"].clone())?;
            resonance_engine::validate_profile(&profile)?;
            std::fs::write(
                PathBuf::from(
                    p["path"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("Missing export path"))?,
                ),
                serde_json::to_vec_pretty(&profile)?,
            )?;
            json!("Profile exported")
        }
        "import_profile" => {
            let path = p
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing import path"))?;
            let data = std::fs::read(path)?;
            anyhow::ensure!(data.len() < 300000, "Profile exceeds 300 KB");
            let profile: Profile = serde_json::from_slice(&data)?;
            resonance_engine::validate_profile(&profile)?;
            serde_json::to_value(profile)?
        }
        "import_css" => serde_json::to_value(
            e.import_spicetify_css(&PathBuf::from(p.as_str().unwrap_or_default()))?,
        )?,
        "import_package" => {
            serde_json::to_value(e.import_package(&PathBuf::from(p.as_str().unwrap_or_default()))?)?
        }
        _ => anyhow::bail!("Unknown command"),
    };
    Ok(out)
}
#[tauri::command]
async fn update_manager(app: tauri::AppHandle, install: bool) -> Result<Value, String> {
    let pref = Engine::default_paths().preferences();
    if pref.update_url.is_empty() {
        return Err("No update feed configured".into());
    }
    let url = pref
        .update_url
        .parse()
        .map_err(|e| format!("Invalid update URL: {e}"))?;
    let updater = app
        .updater_builder()
        .endpoints(vec![url])
        .map_err(|e| e.to_string())?
        .build()
        .map_err(|e| e.to_string())?;
    match updater
        .check()
        .await
        .map_err(|e| format!("Update check failed: {e}"))?
    {
        Some(u) => {
            if install {
                u.download_and_install(|_, _| {}, || {})
                    .await
                    .map_err(|e| format!("Signed update failed: {e}"))?;
                app.restart();
            }
            Ok(json!({"available":true,"version":u.version,"body":u.body}))
        }
        None => Ok(json!({"available":false})),
    }
}
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![engine_command, update_manager])
        .run(tauri::generate_context!())
        .expect("Resonance failed to run");
}
