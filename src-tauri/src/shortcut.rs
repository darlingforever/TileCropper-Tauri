//! 快捷方式创建 —— 用 windows crate 的 IShellLink COM 直接操作
//! 替代 start-menu.js 里通过 PowerShell + WScript.Shell 创建快捷方式
//! 消除了每次 ~400ms 的 powershell.exe 冷启动开销

use std::path::Path;
use windows::core::{Interface, HSTRING, PCWSTR, HRESULT};
use windows::Win32::System::Com::{
    CoInitializeEx, CoCreateInstance, CLSCTX_INPROC_SERVER, CoUninitialize,
    COINIT_APARTMENTTHREADED, IPersistFile,
};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

// S_OK / S_FALSE 在 windows::core 中定义为 HRESULT 常量
const S_OK: HRESULT = HRESULT(0);
const S_FALSE: HRESULT = HRESULT(1);

pub struct ShortcutParams<'a> {
    pub lnk_path: &'a Path,
    pub target_path: &'a Path,
    pub arguments: Option<&'a str>,
    pub icon_path: &'a Path,
    pub work_dir: Option<&'a Path>,
    pub description: Option<&'a str>,
}

/// 创建 .lnk 快捷方式，成功返回 Ok(())
pub fn create_shortcut(params: ShortcutParams) -> Result<(), String> {
    unsafe {
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        // S_FALSE (已初始化) 也算成功
        if hr != S_OK && hr != S_FALSE {
            return Err(format!("CoInitializeEx 失败: hr=0x{:x}", hr.0));
        }

        let result = create_shortcut_inner(&params);
        // 注意：如果 CoInitializeEx 返回 S_FALSE，说明 COM 已被其他代码初始化，
        // 此时不应该调 CoUninitialize，否则可能破坏其他线程的 COM 状态
        if hr == S_OK {
            CoUninitialize();
        }
        result
    }
}

unsafe fn create_shortcut_inner(p: &ShortcutParams) -> Result<(), String> {
    // CoCreateInstance(CLSID_ShellLink) -> IShellLinkW
    let shell_link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
        .map_err(|e| format!("创建 IShellLink 失败: {e}"))?;

    // 目标路径
    let target = HSTRING::from(p.target_path.to_string_lossy().as_ref());
    shell_link.SetPath(PCWSTR(target.as_ptr()))
        .map_err(|e| format!("SetPath 失败: {e}"))?;

    // 工作目录
    let work_dir = match p.work_dir {
        Some(wd) => wd.to_string_lossy().to_string(),
        None => p.target_path.parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_default(),
    };
    let wd_h = HSTRING::from(work_dir.as_str());
    shell_link.SetWorkingDirectory(PCWSTR(wd_h.as_ptr()))
        .map_err(|e| format!("SetWorkingDirectory 失败: {e}"))?;

    // 图标
    let icon = HSTRING::from(p.icon_path.to_string_lossy().as_ref());
    shell_link.SetIconLocation(PCWSTR(icon.as_ptr()), 0)
        .map_err(|e| format!("SetIconLocation 失败: {e}"))?;

    // 参数
    if let Some(args) = p.arguments {
        let args_h = HSTRING::from(args);
        shell_link.SetArguments(PCWSTR(args_h.as_ptr()))
            .map_err(|e| format!("SetArguments 失败: {e}"))?;
    }

    // 描述
    if let Some(desc) = p.description {
        let desc_h = HSTRING::from(desc);
        shell_link.SetDescription(PCWSTR(desc_h.as_ptr()))
            .map_err(|e| format!("SetDescription 失败: {e}"))?;
    }

    // QI -> IPersistFile，保存到 .lnk
    let persist: IPersistFile = shell_link.cast()
        .map_err(|e| format!("QI IPersistFile 失败: {e}"))?;

    let lnk = HSTRING::from(p.lnk_path.to_string_lossy().as_ref());
    persist.Save(PCWSTR(lnk.as_ptr()), true)
        .map_err(|e| format!("保存 .lnk 失败: {e}"))?;

    Ok(())
}

/// 尽力固定到开始屏幕（Windows 通常阻止第三方进程 pin，实测无效）
/// 返回 "ok" / "noverb" / "error"
#[allow(dead_code)]
pub fn pin_to_start(_lnk_path: &Path) -> &'static str {
    // Windows 不允许第三方进程钉磁贴，startpin 动词对非 Explorer 进程调用会被忽略
    "noverb"
}
