//! XML 生成 —— VisualElementsManifest + AppxManifest
//! 移植自 start-menu.js buildVisualElementsManifest / buildSparseAppxManifest

pub fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// 校验颜色格式 #RRGGBB（不引入 regex，直接字符判断）
pub fn sanitize_color(c: &str) -> String {
    let s = if c.starts_with('#') { c.to_string() } else { format!("#{c}") };
    let bytes = s.as_bytes();
    if bytes.len() == 7 && bytes[0] == b'#'
        && bytes[1..].iter().all(|b| b.is_ascii_hexdigit())
    {
        s
    } else {
        "#1f1f1f".to_string()
    }
}

/// 原生开始菜单只认 Square70x70Logo 和 Square150x150Logo 两个槽位
pub fn build_visual_elements_manifest(
    _small_logo: &str,
    _medium_logo: &str,
    background_color: &str,
    foreground_text: &str,
) -> String {
    let color = sanitize_color(background_color);
    let fg = if foreground_text == "dark" { "dark" } else { "light" };
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<Application xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <VisualElements
    ShowNameOnSquare150x150Logo="off"
    ForegroundText="{fg}"
    BackgroundColor="{color}"
    Square70x70Logo="Assets\Square70x70Logo.png"
    Square150x150Logo="Assets\Square150x150Logo.png" />
</Application>
"#
    )
}

/// 包身份清单：四档 Logo 全写，原生菜单对有身份的应用给出全部四档
pub fn build_sparse_appx_manifest(
    identity_name: &str,
    display_name: &str,
    description: &str,
    background_color: &str,
    revision: u32,
    min_build: u32,
) -> String {
    let color = sanitize_color(background_color);
    let name = xml_escape(display_name);
    let desc = xml_escape(description);
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<Package IgnorableNamespaces="uap uap10 rescap"
  xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
  xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
  xmlns:uap10="http://schemas.microsoft.com/appx/manifest/uap/windows10/10"
  xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities">
  <Identity Name="{identity_name}" Publisher="CN=TileCropper" Version="1.0.0.{revision}" ProcessorArchitecture="neutral" />
  <Properties>
    <DisplayName>{name}</DisplayName>
    <PublisherDisplayName>TileCropper</PublisherDisplayName>
    <Logo>Assets\Square150x150Logo.png</Logo>
    <uap10:AllowExternalContent>true</uap10:AllowExternalContent>
  </Properties>
  <Resources />
  <Dependencies>
    <TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.{min_build}.0" MaxVersionTested="10.0.26100.0" />
  </Dependencies>
  <Capabilities>
    <rescap:Capability Name="runFullTrust" />
    <rescap:Capability Name="unvirtualizedResources" />
  </Capabilities>
  <Applications>
    <Application Id="Tile" Executable="TileLauncher.exe" uap10:TrustLevel="mediumIL" uap10:RuntimeBehavior="win32App">
      <uap:VisualElements DisplayName="{name}" Description="{desc}" BackgroundColor="{color}"
        Square150x150Logo="Assets\Square150x150Logo.png" Square44x44Logo="Assets\Square70x70Logo.png"
        AppListEntry="default">
        <uap:DefaultTile
          Square71x71Logo="Assets\Square70x70Logo.png"
          Wide310x150Logo="Assets\Wide310x150Logo.png"
          Square310x310Logo="Assets\Square310x310Logo.png" />
      </uap:VisualElements>
    </Application>
  </Applications>
</Package>
"#
    )
}

/// 磁贴名称清理：去掉文件名非法字符，限长 40
pub fn sanitize_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| !r#"\/*?:"<>|"#.contains(*c) && !c.is_control())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(|c: char| c == '.' || c.is_whitespace())
        .chars()
        .take(40)
        .collect();
    if cleaned.is_empty() { "磁贴图片".to_string() } else { cleaned }
}

/// 从安全名称生成包标识名 TileCropper.Tile.<hash>
pub fn identity_name_for(safe_name: &str) -> String {
    use md5::{Md5, Digest};
    let mut hasher = Md5::new();
    hasher.update(safe_name.as_bytes());
    let hash = hasher.finalize();
    let hex: String = hash.iter().take(6).map(|b| format!("{:02x}", b)).collect();
    format!("TileCropper.Tile.{hex}")
}
