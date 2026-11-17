//! 磁贴主流程 —— addIconTile / addFullBleedTile / addStartTileFromPayload
//! 移植自 start-menu.js 的核心编排逻辑

use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::ico::{build_ico, decode_data_url};
use crate::launcher::{ensure_launcher, data_dir, LAUNCHER_NAME};
use crate::manifest::{build_visual_elements_manifest, sanitize_name};
use crate::msix::register_sparse_tile;
use crate::shortcut::{create_shortcut, ShortcutParams};

const TILE_SLOTS: &[(&str, &str, u32, u32)] = &[
    ("small", "Square70x70Logo", 70, 70),
    ("medium", "Square150x150Logo", 150, 150),
    ("wide", "Wide310x150Logo", 310, 150),
    ("large", "Square310x310Logo", 310, 310),
];

const LAUNCHER_MANIFEST: &str = "TileLauncher.VisualElementsManifest.xml";

/// 开始菜单程序目录（统一放到 TileCropper 子文件夹，避免散落在 Programs 根目录）
fn start_menu_programs_dir() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_default();
    PathBuf::from(appdata)
        .join("Microsoft").join("Windows").join("Start Menu").join("Programs")
        .join("TileCropper")
}

/// 应用视觉缓存目录
fn app_shortcuts_cache_dir(safe_name: &str) -> PathBuf {
    let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
    PathBuf::from(local).join("Microsoft").join("Windows").join("Application Shortcuts").join(safe_name)
}

// ---------- IPC 数据结构 ----------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TileVariant {
    pub scale: u32,
    pub data_url: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IcoImage {
    pub size: u32,
    pub data_url: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddTilePayload {
    pub name: String,
    pub png_data_url: String,
    pub ico_images: Vec<IcoImage>,
    pub tiles: std::collections::HashMap<String, Vec<TileVariant>>,
    pub background_color: String,
    pub foreground_text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SparseInfo {
    pub ok: bool,
    pub error: Option<String>,
    pub reason: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddTileResult {
    pub ok: bool,
    pub error: Option<String>,
    pub name: Option<String>,
    pub mode: Option<String>,
    pub pin: Option<String>,
    pub pinned_via_package: bool,
    pub sparse: Option<SparseInfo>,
}

impl AddTileResult {
    fn ok_full(name: &str, pinned_via_package: bool, sparse: Option<SparseInfo>) -> Self {
        Self { ok: true, error: None, name: Some(name.into()), mode: Some("fullbleed".into()),
               pin: None, pinned_via_package, sparse }
    }
    fn ok_icon(name: &str) -> Self {
        Self { ok: true, error: None, name: Some(name.into()), mode: Some("icon".into()),
               pin: None, pinned_via_package: false, sparse: None }
    }
    fn fail(msg: &str) -> Self {
        Self { ok: false, error: Some(msg.into()), name: None, mode: None,
               pin: None, pinned_via_package: false, sparse: None }
    }
}

// ---------- 图标式（兜底） ----------

fn add_icon_tile(
    name: &str, png_buffer: &[u8], ico_images: &[(u32, Vec<u8>)],
) -> AddTileResult {
    let safe_name = sanitize_name(name);
    let tile_dir = data_dir().join("StartTiles");
    let menu_dir = start_menu_programs_dir();

    if let Err(e) = fs::create_dir_all(&tile_dir).and_then(|_| fs::create_dir_all(&menu_dir)) {
        return AddTileResult::fail(&format!("创建目录失败: {e}"));
    }

    let png_path = tile_dir.join(format!("{safe_name}.png"));
    let ico_path = tile_dir.join(format!("{safe_name}.ico"));
    let lnk_path = menu_dir.join(format!("{safe_name}.lnk"));

    if let Err(e) = fs::write(&png_path, png_buffer) {
        return AddTileResult::fail(&format!("写 PNG 失败: {e}"));
    }
    let ico = match build_ico(ico_images) {
        Ok(d) => d,
        Err(e) => return AddTileResult::fail(&e),
    };
    if let Err(e) = fs::write(&ico_path, &ico) {
        return AddTileResult::fail(&format!("写 ICO 失败: {e}"));
    }

    let params = ShortcutParams {
        lnk_path: &lnk_path, target_path: &png_path,
        arguments: None, icon_path: &ico_path,
        work_dir: Some(&tile_dir),
        description: Some(&format!("磁贴图片：{safe_name}")),
    };
    if let Err(e) = create_shortcut(params) {
        return AddTileResult::fail(&e);
    }

    AddTileResult::ok_icon(&safe_name)
}

// ---------- 铺满式 ----------

fn add_full_bleed_tile(payload: &AddTilePayload, png_buffer: &[u8], ico_images: &[(u32, Vec<u8>)]) -> AddTileResult {
    let safe_name = sanitize_name(&payload.name);
    let root = data_dir().join("StartTiles");
    let menu_dir = start_menu_programs_dir();
    let tile_dir = root.join(&safe_name);
    let assets_dir = tile_dir.join("Assets");

    let launcher = match ensure_launcher() {
        Some(p) => p,
        None => {
            // 编译不出启动器，退回图标式
            return add_icon_tile(&payload.name, png_buffer, ico_images);
        }
    };

    if let Err(e) = fs::create_dir_all(&assets_dir).and_then(|_| fs::create_dir_all(&menu_dir)) {
        return AddTileResult::fail(&format!("创建目录失败: {e}"));
    }

    // 复制启动器（manifest 必须与 exe 同名同目录）
    let launcher_path = tile_dir.join(LAUNCHER_NAME);
    if let Err(e) = fs::copy(&launcher, &launcher_path) {
        return AddTileResult::fail(&format!("复制启动器失败: {e}"));
    }

    // 写四档素材
    let mut available: Vec<&str> = Vec::new();
    let mut written: Vec<(String, PathBuf)> = Vec::new();

    for (slot, base, _w, _h) in TILE_SLOTS {
        let variants = match payload.tiles.get(*slot) {
            Some(v) if !v.is_empty() => v,
            _ => continue,
        };
        available.push(slot);

        for variant in variants {
            let suffix = if variant.scale == 100 { String::new() }
                         else { format!(".scale-{}", variant.scale) };
            let file_path = assets_dir.join(format!("{base}{suffix}.png"));
            match decode_data_url(&variant.data_url) {
                Ok(buf) => {
                    if let Err(e) = fs::write(&file_path, &buf) {
                        return AddTileResult::fail(&format!("写素材失败: {e}"));
                    }
                    written.push((format!("{slot}@{}", variant.scale), file_path));
                }
                Err(e) => return AddTileResult::fail(&e),
            }
        }
    }

    // 没有中档素材 = 没有磁贴定义，退回图标式
    if !available.contains(&"medium") {
        return add_icon_tile(&payload.name, png_buffer, ico_images);
    }

    // 清掉这次没提供的档位残留
    if let Ok(entries) = fs::read_dir(&assets_dir) {
        let file_list: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        for (slot, base, _, _) in TILE_SLOTS {
            if available.contains(slot) { continue; }
            for e in &file_list {
                if let Some(fname) = e.file_name().to_str() {
                    if fname.starts_with(&format!("{base}.png")) || fname.starts_with(&format!("{base}.scale-")) {
                        let _ = fs::remove_file(e.path());
                    }
                }
            }
        }
    }

    // 写 manifest
    let manifest_xml = build_visual_elements_manifest(
        "Assets\\Square70x70Logo.png",
        "Assets\\Square150x150Logo.png",
        &payload.background_color,
        &payload.foreground_text,
    );
    if let Err(e) = fs::write(tile_dir.join(LAUNCHER_MANIFEST), manifest_xml) {
        return AddTileResult::fail(&format!("写 manifest 失败: {e}"));
    }

    // 写 PNG + ICO
    let png_path = tile_dir.join(format!("{safe_name}.png"));
    let ico_path = tile_dir.join(format!("{safe_name}.ico"));
    if let Err(e) = fs::write(&png_path, png_buffer) {
        return AddTileResult::fail(&format!("写 PNG 失败: {e}"));
    }
    let ico = match build_ico(ico_images) {
        Ok(d) => d,
        Err(e) => return AddTileResult::fail(&e),
    };
    if let Err(e) = fs::write(&ico_path, &ico) {
        return AddTileResult::fail(&format!("写 ICO 失败: {e}"));
    }

    // 创建快捷方式
    let lnk_path = menu_dir.join(format!("{safe_name}.lnk"));
    let png_arg = format!("\"{}\"", png_path.display());
    let params = ShortcutParams {
        lnk_path: &lnk_path, target_path: &launcher_path,
        arguments: Some(&png_arg), icon_path: &ico_path,
        work_dir: Some(&tile_dir),
        description: Some(&format!("磁贴：{safe_name}")),
    };
    if let Err(e) = create_shortcut(params) {
        return AddTileResult::fail(&e);
    }

    // 清视觉缓存
    let cache_dir = app_shortcuts_cache_dir(&safe_name);
    let _ = fs::remove_dir_all(&cache_dir);

    // 包身份：宽 / 大走 sparse package
    let mut sparse_info = None;
    let mut pinned_via_package = false;
    if available.contains(&"wide") && available.contains(&"large") {
        let slots_ref: Vec<&str> = available.clone();
        let sparse = register_sparse_tile(&tile_dir, &safe_name, &payload.background_color, &slots_ref);
        sparse_info = Some(SparseInfo {
            ok: sparse.ok,
            error: sparse.error,
            reason: sparse.reason,
        });
        if sparse.ok {
            // 包身份成功：删 .lnk 避免双条目
            let _ = fs::remove_file(&lnk_path);
            let _ = fs::remove_dir_all(&cache_dir);
            pinned_via_package = true;
        }
    }

    AddTileResult::ok_full(&safe_name, pinned_via_package, sparse_info)
}

// ---------- 主入口 ----------

pub fn add_start_tile_from_payload(payload: AddTilePayload) -> AddTileResult {
    if payload.ico_images.is_empty() {
        return AddTileResult::fail("缺少图标数据");
    }

    let png_buffer = match decode_data_url(&payload.png_data_url) {
        Ok(b) => b,
        Err(e) => return AddTileResult::fail(&e),
    };

    let mut ico_images: Vec<(u32, Vec<u8>)> = Vec::with_capacity(payload.ico_images.len());
    for img in &payload.ico_images {
        match decode_data_url(&img.data_url) {
            Ok(buf) => ico_images.push((img.size, buf)),
            Err(e) => return AddTileResult::fail(&e),
        }
    }

    let has_tiles = payload.tiles.get("medium").map_or(false, |v| !v.is_empty());
    if !has_tiles {
        return add_icon_tile(&payload.name, &png_buffer, &ico_images);
    }

    add_full_bleed_tile(&payload, &png_buffer, &ico_images)
}
