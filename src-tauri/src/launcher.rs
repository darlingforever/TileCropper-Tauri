//! C# 启动器编译 —— 移植自 start-menu.js ensureLauncher
//! 用系统自带的 csc.exe 现场编译一个几百字节的启动器
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub const LAUNCHER_NAME: &str = "TileLauncher.exe";
pub const LAUNCHER_VERSION: u32 = 2;

const LAUNCHER_SOURCE: &str = r#"using System;
using System.Diagnostics;
using System.IO;

internal static class TileLauncher
{
    [STAThread]
    private static void Main(string[] args)
    {
        try
        {
            if (args.Length > 0 && File.Exists(args[0]))
            {
                Process.Start(args[0]);
                return;
            }
            string[] pngs = Directory.GetFiles(AppDomain.CurrentDomain.BaseDirectory, "*.png");
            if (pngs.Length > 0)
            {
                Process.Start(pngs[0]);
            }
        }
        catch
        {
        }
    }
}
"#;

/// 数据目录：%APPDATA%\tile_cropper
pub fn data_dir() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_default();
    PathBuf::from(appdata).join("tile_cropper")
}

/// 编译启动器（一次编译，所有磁贴共用），返回 exe 路径
pub fn ensure_launcher() -> Option<PathBuf> {
    let bin_dir = data_dir().join("bin");
    let exe_path = bin_dir.join(LAUNCHER_NAME);
    let version_path = bin_dir.join("launcher-version.txt");

    // 版本匹配且 exe 存在 -> 直接复用
    if let Ok(saved) = fs::read_to_string(&version_path) {
        if saved.trim().parse::<u32>().unwrap_or(0) == LAUNCHER_VERSION {
            if exe_path.exists() {
                return Some(exe_path);
            }
        }
    }

    fs::create_dir_all(&bin_dir).ok()?;
    let source_path = bin_dir.join("TileLauncher.cs");
    fs::write(&source_path, LAUNCHER_SOURCE).ok()?;

    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
    let candidates = [
        PathBuf::from(&windir).join("Microsoft.NET").join("Framework64").join("v4.0.30319"),
        PathBuf::from(&windir).join("Microsoft.NET").join("Framework").join("v4.0.30319"),
    ];

    for csc_dir in &candidates {
        let csc = csc_dir.join("csc.exe");
        if !csc.exists() { continue; }

        let output = Command::new(&csc)
            .args(["/nologo", "/target:winexe", "/optimize+",
                   &format!("/out:{}", LAUNCHER_NAME), "TileLauncher.cs"])
            .current_dir(&bin_dir)
            .output();

        if let Ok(out) = output {
            if out.status.success() && exe_path.exists() {
                let _ = fs::write(&version_path, LAUNCHER_VERSION.to_string());
                return Some(exe_path);
            }
        }
    }
    None
}
