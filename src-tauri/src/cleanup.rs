//! 清理残留包身份应用 —— 移植自 start-menu.js cleanupTilePackages
use std::fs;
use std::path::PathBuf;
use serde::Serialize;
use crate::launcher::data_dir;
use crate::manifest::identity_name_for;
use crate::powershell::{ps_run, ps_string};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupResult {
    pub removed: Vec<String>,
    pub failed: Vec<String>,
    pub scanned: usize,
}

struct PkgInfo {
    name: String,
    #[allow(dead_code)]
    location: String,
}

/// 列出已注册的 TileCropper.Tile.* 包
fn list_registered_packages() -> Vec<PkgInfo> {
    let script = r#"
$ErrorActionPreference = 'Stop'
$pkgs = Get-AppxPackage -Name 'TileCropper.Tile.*' -ErrorAction SilentlyContinue
foreach ($p in $pkgs) {
    Write-Output ("NAME=" + $p.Name)
    Write-Output ("LOC=" + $p.InstallLocation)
}
"#;
    let result = ps_run(script, 30000);
    if !result.ok { return Vec::new(); }

    let mut out = Vec::new();
    let mut cur_name: Option<String> = None;
    for line in result.stdout.lines() {
        if let Some(name) = line.strip_prefix("NAME=") {
            cur_name = Some(name.trim().to_string());
        } else if let Some(loc) = line.strip_prefix("LOC=") {
            if let Some(name) = cur_name.take() {
                out.push(PkgInfo { name, location: loc.trim().to_string() });
            }
        }
    }
    out
}

/// 把包名和 StartTiles 目录名对应
fn map_packages_to_names(packages: Vec<PkgInfo>) -> Vec<(PkgInfo, Option<String>, bool)> {
    let root = data_dir().join("StartTiles");
    let dirs: Vec<String> = match fs::read_dir(&root) {
        Ok(rd) => rd.filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect(),
        Err(_) => Vec::new(),
    };

    packages.into_iter().map(|pkg| {
        let mut matched: Option<String> = None;
        let mut exists = false;
        for dir in &dirs {
            if identity_name_for(dir) == pkg.name {
                matched = Some(dir.clone());
                exists = root.join(dir).exists();
                break;
            }
        }
        (pkg, matched, exists)
    }).collect()
}

fn remove_appx_package(identity_name: &str) -> bool {
    let name = ps_string(identity_name);
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
Get-AppxPackage -Name {name} -ErrorAction SilentlyContinue | Remove-AppxPackage -ErrorAction Stop
Write-Output 'REMOVED'
"#
    );
    let result = ps_run(&script, 60000);
    result.ok && result.stdout.contains("REMOVED")
}

fn clean_artifacts(safe_name: &str) {
    // 新路径：Programs\TileCropper\
    let lnk = start_menu_programs_dir().join(format!("{safe_name}.lnk"));
    let _ = fs::remove_file(&lnk);
    // 旧路径兼容：Programs\ 根目录（旧版直接放根目录）
    let appdata = std::env::var("APPDATA").unwrap_or_default();
    let old_lnk = PathBuf::from(&appdata)
        .join("Microsoft").join("Windows").join("Start Menu").join("Programs")
        .join(format!("{safe_name}.lnk"));
    let _ = fs::remove_file(&old_lnk);
    // 视觉缓存
    let cache = PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default())
        .join("Microsoft").join("Windows").join("Application Shortcuts").join(safe_name);
    let _ = fs::remove_dir_all(&cache);
}

/// 开始菜单程序目录（与 tile.rs 保持一致：TileCropper 子文件夹）
fn start_menu_programs_dir() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_default();
    PathBuf::from(appdata)
        .join("Microsoft").join("Windows").join("Start Menu").join("Programs")
        .join("TileCropper")
}

/// 主清理入口：name=None 扫描所有残留；name=Some 只处理指定那个
pub fn cleanup_tile_packages(name: Option<&str>) -> CleanupResult {
    let packages = list_registered_packages();
    let mapping = map_packages_to_names(packages);
    let scanned = mapping.len();

    let targets: Vec<&(PkgInfo, Option<String>, bool)> = match name {
        Some(n) => mapping.iter().filter(|(_, sn, _)| sn.as_deref() == Some(n)).collect(),
        None => mapping.iter().filter(|(_, _, exists)| !exists).collect(),
    };

    let mut removed = Vec::new();
    let mut failed = Vec::new();

    for (pkg, safe_name, _) in targets {
        if remove_appx_package(&pkg.name) {
            if let Some(sn) = safe_name {
                clean_artifacts(sn);
            }
            removed.push(safe_name.clone().unwrap_or_else(|| pkg.name.clone()));
        } else {
            failed.push(safe_name.clone().unwrap_or_else(|| pkg.name.clone()));
        }
    }

    CleanupResult { removed, failed, scanned }
}
