//! 证书管理 —— 自签名代码签名证书
//! 生成用 PowerShell（New-SelfSignedCertificate），一次性操作，所有磁贴共用
//! 信任导入需要 UAC（通过 PowerShell Start-Process -Verb RunAs）

use std::fs;
use std::path::PathBuf;
use crate::launcher::data_dir;
use crate::powershell::{ps_run, ps_string};

const CERT_SUBJECT: &str = "CN=TileCropper";

pub struct CertInfo {
    pub pfx_path: PathBuf,
    pub cer_path: PathBuf,
    pub password: String,
}

/// 生成或复用自签名证书
pub fn ensure_identity_cert() -> Option<CertInfo> {
    let cert_dir = data_dir().join("certs");
    let pfx_path = cert_dir.join("TileCropper.pfx");
    let cer_path = cert_dir.join("TileCropper.cer");
    let meta_path = cert_dir.join("cert.json");

    // 复用已有证书
    if let Ok(meta) = fs::read_to_string(&meta_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&meta) {
            if let Some(pw) = json["password"].as_str() {
                if pfx_path.exists() && cer_path.exists() {
                    return Some(CertInfo {
                        pfx_path, cer_path,
                        password: pw.to_string(),
                    });
                }
            }
        }
    }

    fs::create_dir_all(&cert_dir).ok()?;
    let password = gen_password();
    let pfx_str = pfx_path.to_string_lossy().replace('\'', "''");
    let cer_str = cer_path.to_string_lossy().replace('\'', "''");

    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
$secure = ConvertTo-SecureString -String '{password}' -Force -AsPlainText
$cert = New-SelfSignedCertificate -Type Custom -Subject '{CERT_SUBJECT}' -KeyUsage DigitalSignature -KeyAlgorithm RSA -KeyLength 2048 -FriendlyName 'TileCropper' -CertStoreLocation 'Cert:\CurrentUser\My' -TextExtension @('2.5.29.37={{text}}1.3.6.1.5.5.7.3.3','2.5.29.19={{text}}')
Export-PfxCertificate -Cert $cert -FilePath '{pfx_str}' -Password $secure | Out-Null
Export-Certificate -Cert $cert -FilePath '{cer_str}' | Out-Null
Write-Output 'CERT_OK'
"#,
        password = password,
    );

    let result = ps_run(&script, 30000);
    if !result.ok || !result.stdout.contains("CERT_OK") {
        return None;
    }

    let meta = serde_json::json!({ "password": password });
    let _ = fs::write(&meta_path, serde_json::to_string_pretty(&meta).unwrap_or_default());

    Some(CertInfo { pfx_path, cer_path, password })
}

/// 证书是否已在指定存储里
pub fn cert_in_store(cer_path: &PathBuf, store: &str) -> bool {
    let cer_str = ps_string(&cer_path.to_string_lossy());
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
$cert = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2({cer_str})
$hit = Get-ChildItem {store} -ErrorAction SilentlyContinue | Where-Object {{ $_.Thumbprint -eq $cert.Thumbprint }}
if ($hit) {{ Write-Output 'FOUND' }} else {{ Write-Output 'MISSING' }}
"#
    );
    let result = ps_run(&script, 20000);
    result.ok && result.stdout.contains("FOUND")
}

/// 导入证书到本机受信任人存储（需要一次 UAC）
pub fn trust_cert_for_appx(cer_path: &PathBuf) -> Result<bool, String> {
    let store = "Cert:\\LocalMachine\\TrustedPeople";
    if cert_in_store(cer_path, store) {
        return Ok(true);
    }

    let cer_str = ps_string(&cer_path.to_string_lossy());
    let inner = format!(
        "try {{ Import-Certificate -FilePath {cer_str} -CertStoreLocation {store} | Out-Null; exit 0 }} catch {{ exit 1 }}"
    );
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
try {{
    $p = Start-Process powershell.exe -Verb RunAs -Wait -PassThru -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-Command', {})
    if ($p.ExitCode -ne 0) {{ Write-Output ('ELEVATE_FAIL:' + $p.ExitCode) }} else {{ Write-Output 'ELEVATE_DONE' }}
}} catch {{
    Write-Output 'ELEVATE_CANCELLED'
}}
"#,
        ps_string(&inner),
    );

    let result = ps_run(&script, 180000);
    if !result.ok {
        return Err(result.error.unwrap_or_else(|| "启动管理员授权失败".into()));
    }
    if result.stdout.contains("ELEVATE_CANCELLED") {
        return Err("用户取消了管理员授权".into());
    }
    if !result.stdout.contains("ELEVATE_DONE") {
        return Err(result.stdout);
    }
    if cert_in_store(cer_path, store) {
        Ok(true)
    } else {
        Err("证书未出现在本机受信任人存储".into())
    }
}

fn gen_password() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    format!("{:024x}", now.as_nanos())
}
