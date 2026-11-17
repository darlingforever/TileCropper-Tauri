//! PowerShell 执行器 —— 仍用于证书管理和 Appx 注册（这些操作没有好的 Rust 原生替代）
//! 但把多次调用合并，减少 powershell.exe 冷启动次数
use std::process::Command;

pub struct PsResult {
    pub ok: bool,
    pub stdout: String,
    #[allow(dead_code)]
    pub stderr: String,
    pub error: Option<String>,
}

/// 用 -EncodedCommand 传入（UTF-16LE + base64），中文名称和反斜杠路径不用转义
pub fn ps_run(script: &str, _timeout_ms: u64) -> PsResult {
    let full = format!(
        "[Console]::OutputEncoding=[System.Text.Encoding]::UTF8\n{}",
        script
    );
    let encoded = base64_encode_utf16le(&full);

    let mut cmd = Command::new("powershell.exe");
    cmd.args([
        "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass",
        "-EncodedCommand", &encoded,
    ]);

    let child = cmd.output();
    match child {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            let ok = out.status.success();
            PsResult { ok, stdout, stderr, error: None }
        }
        Err(e) => PsResult { ok: false, stdout: String::new(), stderr: String::new(), error: Some(e.to_string()) },
    }
}

/// PowerShell 字符串转义：单引号 -> 两个单引号，用单引号包裹
pub fn ps_string(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn base64_encode_utf16le(s: &str) -> String {
    use base64::Engine;
    let utf16: Vec<u16> = s.encode_utf16().collect();
    let bytes: Vec<u8> = utf16
        .iter()
        .flat_map(|w| w.to_le_bytes())
        .collect();
    base64::engine::general_purpose::STANDARD.encode(&bytes)
}
