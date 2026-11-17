//! TileCropper Tauri 后端入口
//! 暴露两个命令给前端：add_start_tile / cleanup_tiles
//! pickImage 由前端的 tauri-plugin-dialog 直接处理，无需 Rust 命令

mod ico;
mod manifest;
mod powershell;
mod launcher;
mod shortcut;
mod cert;
mod msix;
mod tile;
mod cleanup;

use tile::{AddTilePayload, AddTileResult};
use cleanup::CleanupResult;
use base64::{engine::general_purpose::STANDARD, Engine};

/// 读取本地图片文件，返回 base64 data URL。
/// 解决 Tauri asset 协议跨域导致 Canvas 污染（toDataURL SecurityError）的问题：
/// data URL 与页面同源，画到 Canvas 不会被标记 tainted。
#[tauri::command]
fn read_image_as_data_url(path: String) -> Result<String, String> {
    let ext = path
        .rsplit('.')
        .next()
        .map(|e| e.to_lowercase())
        .unwrap_or_else(|| "png".into());
    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "webp" => "image/webp",
        _ => "image/png",
    };
    let bytes = std::fs::read(&path).map_err(|e| format!("读取图片失败：{e}"))?;
    let b64 = STANDARD.encode(&bytes);
    Ok(format!("data:{mime};base64,{b64}"))
}

#[tauri::command]
fn add_start_tile(payload: AddTilePayload) -> AddTileResult {
    tile::add_start_tile_from_payload(payload)
}

#[tauri::command]
fn cleanup_tiles(name: Option<String>) -> CleanupResult {
    cleanup::cleanup_tile_packages(name.as_deref())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            read_image_as_data_url,
            add_start_tile,
            cleanup_tiles
        ])
        .run(tauri::generate_context!())
        .expect("启动 Tauri 应用失败");
}
