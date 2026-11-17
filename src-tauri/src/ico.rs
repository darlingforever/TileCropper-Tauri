//! ICO 二进制组装 —— 移植自 start-menu.js buildIco
//! ICO 格式：6 字节头 + 每张 16 字节目录项 + 各自的 PNG 数据
//! Vista 以后 ICO 允许直接内嵌 PNG，不需要自己写 BMP 编码

/// 输入：(尺寸, PNG buffer) 列表；输出：完整 ICO 字节
pub fn build_ico(images: &[(u32, Vec<u8>)]) -> Result<Vec<u8>, String> {
    let mut list: Vec<(u32, Vec<u8>)> = images
        .iter()
        .filter(|(_, buf)| !buf.is_empty())
        .map(|(size, buf)| (*size, buf.clone()))
        .collect();
    list.sort_by_key(|(s, _)| *s);

    if list.is_empty() {
        return Err("没有可用的图标数据".into());
    }

    // 头：reserved(2) + type(2, =1) + count(2)
    let count = list.len() as u16;
    let mut out: Vec<u8> = Vec::with_capacity(6 + 16 * list.len() + list.iter().map(|(_, b)| b.len()).sum::<usize>());
    out.extend_from_slice(&0u16.to_le_bytes());      // reserved
    out.extend_from_slice(&1u16.to_le_bytes());      // type = ICO
    out.extend_from_slice(&count.to_le_bytes());     // 图片数量

    // 目录项：每项 16 字节，记录尺寸 + offset
    let mut offset = 6u32 + 16 * list.len() as u32;
    for (size, buf) in &list {
        let s = if *size >= 256 { 0u8 } else { *size as u8 }; // ICO 规范：256 编码为 0
        out.push(s);                     // 宽
        out.push(s);                     // 高
        out.push(0);                     // 调色板数
        out.push(0);                     // 保留
        out.extend_from_slice(&1u16.to_le_bytes());         // 颜色平面数
        out.extend_from_slice(&32u16.to_le_bytes());        // 位深
        out.extend_from_slice(&(buf.len() as u32).to_le_bytes()); // 数据大小
        out.extend_from_slice(&offset.to_le_bytes());       // 数据偏移
        offset += buf.len() as u32;
    }

    // 各图片数据
    for (_, buf) in &list {
        out.extend_from_slice(buf);
    }

    Ok(out)
}

/// 从 data:image/png;base64,... 解码出 PNG buffer
pub fn decode_data_url(data_url: &str) -> Result<Vec<u8>, String> {
    let prefix = data_url.split(',').next().unwrap_or("");
    if !prefix.starts_with("data:image/") || !prefix.contains("base64") {
        return Err("图像数据格式不正确".into());
    }
    let b64 = data_url.splitn(2, ',').nth(1).unwrap_or("");
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .map_err(|e| format!("base64 解码失败: {e}"))
}
