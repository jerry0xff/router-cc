//! 智能路由 Tauri 命令

use crate::proxy::intelligent_router::IntelligentRoutingSettings;
use crate::store::AppState;
use tauri::State;

/// 获取指定应用的智能路由设置
#[tauri::command]
pub async fn get_intelligent_routing_settings(
    app_type: String,
    state: State<'_, AppState>,
) -> Result<IntelligentRoutingSettings, String> {
    let key = format!("intelligent_routing_{app_type}");
    let settings = state
        .db
        .get_setting(&key)
        .map_err(|e| e.to_string())?
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default();
    Ok(settings)
}

/// 保存指定应用的智能路由设置
#[tauri::command]
pub async fn update_intelligent_routing_settings(
    app_type: String,
    settings: IntelligentRoutingSettings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let key = format!("intelligent_routing_{app_type}");
    let json = serde_json::to_string(&settings).map_err(|e| e.to_string())?;
    state.db.set_setting(&key, &json).map_err(|e| e.to_string())?;
    Ok(())
}
