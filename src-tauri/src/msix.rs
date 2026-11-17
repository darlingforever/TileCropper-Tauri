//! Sparse package（包身份）—— 打包、签名、注册
//! 让原生开始菜单认宽 / 大磁贴
//! 移植自 start-menu.js registerSparseTile

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::cert::{ensure_identity_cert, trust_cert_for_appx};
use crate::manifest::{build_sparse_appx_manifest, identity_name_for};
use crate::powershell::{ps_run, ps_string};

pub const SPARSE_MIN_BUILD: u32 = 19041;

pub struct SparseResult {
    pub ok: bool,
    #[allow(dead_code)]
    pub identity_name: String,
    pub reason: Option<String>,
    pub error: Option<String>,
}

impl SparseResult {
    fn fail(reason: &str, error: String) -> Self {
        Self { ok: false, identity_name: String::new(), reason: Some(reason.into()), error: Some(error) }
    }
}

/// 获取 Windows 版本号（os.release 形如 10.0.19045 -> 19045）
pub fn windows_build_number() -> u32 {
    // 通过 GetVersionExW 或注册表获取，这里用 PowerShell 一次性查
    let result = ps_run("[System.Environment]::OSVersion.Version.Build", 10000);
    result.stdout.trim().parse::<u32>().unwrap_or(0)
}

/// 找 Windows SDK 的 MakeAppx / Signtool
pub fn find_sdk_tools() -> Option<(PathBuf, PathBuf)> {
    let roots = [
        std::env::var("ProgramFiles(x86)").unwrap_or_else(|_| "C:\\Program Files (x86)".into()),
        std::env::var("ProgramFiles").unwrap_or_else(|_| "C:\\Program Files".into()),
    ];

    for root in &roots {
        let bin_root = PathBuf::from(root).join("Windows Kits").join("10").join("bin");
        let mut versions: Vec<_> = match fs::read_dir(&bin_root) {
            Ok(rd) => rd.filter_map(|e| e.ok())
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|n| n.starts_with("10."))
                .collect(),
            Err(_) => continue,
        };
        versions.sort();
        versions.reverse();

        for ver in versions {
            let makeappx = bin_root.join(&ver).join("x64").join("makeappx.exe");
            let signtool = bin_root.join(&ver).join("x64").join("signtool.exe");
            if makeappx.exists() && signtool.exists() {
                return Some((makeappx, signtool));
            }
        }
    }
    None
}

/// 查已注册包的版本修订号
pub fn current_package_revision(identity_name: &str) -> Option<u32> {
    let name = ps_string(identity_name);
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
$p = Get-AppxPackage -Name {name} -ErrorAction SilentlyContinue
if ($p) {{ Write-Output ('REV:' + ([version]$p.Version).Revision) }} else {{ Write-Output 'REV:NONE' }}
"#
    );
    let result = ps_run(&script, 20000);
    if !result.ok { return None; }
    if let Some(rest) = result.stdout.strip_prefix("REV:") {
        if rest.trim() == "NONE" { return None; }
        return rest.trim().parse::<u32>().ok();
    }
    None
}

/// 打包、签名、注册 sparse package
pub fn register_sparse_tile(
    tile_dir: &Path,
    safe_name: &str,
    background_color: &str,
    slots: &[&str],
) -> SparseResult {
    if windows_build_number() < SPARSE_MIN_BUILD {
        return SparseResult::fail("old-os", "系统低于 Windows 10 2004".into());
    }
    if !slots.contains(&"wide") || !slots.contains(&"large") {
        return SparseResult::fail("no-slots", "缺少宽 / 大素材".into());
    }

    let (makeappx, signtool) = match find_sdk_tools() {
        Some(t) => t,
        None => return SparseResult::fail("no-sdk", "未找到 Windows SDK 的 MakeAppx / Signtool".into()),
    };

    let cert = match ensure_identity_cert() {
        Some(c) => c,
        None => return SparseResult::fail("no-cert", "生成签名证书失败".into()),
    };
    if let Err(e) = trust_cert_for_appx(&cert.cer_path) {
        return SparseResult::fail("cert-untrusted", e);
    }

    let identity_name = identity_name_for(safe_name);
    let existing_rev = current_package_revision(&identity_name);
    let sparse_dir = tile_dir.join("Identity");
    let sparse_assets = sparse_dir.join("Assets");
    let _ = fs::create_dir_all(&sparse_assets);

    // 复制素材到 sparse 包目录
    let src_assets = tile_dir.join("Assets");
    if let Ok(entries) = fs::read_dir(&sparse_assets) {
        for e in entries.flatten() {
            let _ = fs::remove_file(e.path());
        }
    }
    if let Ok(entries) = fs::read_dir(&src_assets) {
        for e in entries.flatten() {
            let _ = fs::copy(e.path(), sparse_assets.join(e.file_name()));
        }
    }

    // 版本修订号
    let (revision, remove_first) = match existing_rev {
        Some(r) if r < 65535 => (r + 1, false),
        Some(_) => (1, true),
        None => {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as u32;
            (std::cmp::max(1, ts % 65535), false)
        }
    };

    let manifest_xml = build_sparse_appx_manifest(
        &identity_name, safe_name, &format!("磁贴：{safe_name}"),
        background_color, revision, SPARSE_MIN_BUILD,
    );
    if let Err(e) = fs::write(sparse_dir.join("AppxManifest.xml"), manifest_xml) {
        return SparseResult::fail("manifest", e.to_string());
    }

    // MakeAppx pack
    let msix_path = tile_dir.join("TileIdentity.msix");
    let pack = Command::new(&makeappx)
        .args(["pack", "/o", "/d"])
        .arg(&sparse_dir)
        .args(["/nv", "/p"])
        .arg(&msix_path)
        .output();

    match pack {
        Ok(out) if out.status.success() => {}
        Ok(out) => {
            let detail = String::from_utf8_lossy(&out.stderr).trim().to_string();
            return SparseResult::fail("pack", detail);
        }
        Err(e) => return SparseResult::fail("pack", e.to_string()),
    }

    // Signtool sign
    let sign = Command::new(&signtool)
        .args(["sign", "/fd", "SHA256", "/f"])
        .arg(&cert.pfx_path)
        .args(["/p"])
        .arg(&cert.password)
        .arg(&msix_path)
        .output();

    match sign {
        Ok(out) if out.status.success() => {}
        Ok(out) => {
            let detail = String::from_utf8_lossy(&out.stderr).trim().to_string();
            return SparseResult::fail("sign", detail);
        }
        Err(e) => return SparseResult::fail("sign", e.to_string()),
    }

    // 版本回绕时先注销
    if remove_first {
        let name = ps_string(&identity_name);
        let _ = ps_run(
            &format!("Get-AppxPackage -Name {name} -ErrorAction SilentlyContinue | Remove-AppxPackage -ErrorAction SilentlyContinue"),
            30000,
        );
    }

    // Add-AppxPackage 注册
    let msix_str = ps_string(&msix_path.to_string_lossy());
    let tile_str = ps_string(&tile_dir.to_string_lossy());
    let reg_script = format!(
        r#"
$ErrorActionPreference = 'Stop'
try {{
    Add-AppxPackage -Path {msix_str} -ExternalLocation {tile_str} -ErrorAction Stop
    Write-Output 'REG_OK'
}} catch {{
    Write-Output ('REG_FAIL: ' + $_.Exception.Message)
    exit 1
}}"#
    );
    let reg = ps_run(&reg_script, 60000);
    if !reg.stdout.contains("REG_OK") {
        let err = reg.stdout.strip_prefix("REG_FAIL: ")
            .unwrap_or(&reg.stdout)
            .to_string();
        return SparseResult::fail("register", err);
    }

    SparseResult { ok: true, identity_name, reason: None, error: None }
}
