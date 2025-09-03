use std::{path::Path, time::Duration};

use anyhow::Result;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use tokio::fs;

/// 创建进度条
pub fn create_progress_bar(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(100));
    
    // 处理模板Result
    let style = ProgressStyle::default_spinner()
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
        .template("{spinner} {msg}")
        .unwrap_or_else(|_| ProgressStyle::default_spinner()); // 失败时使用默认样式
    
    pb.set_style(style);
    pb.set_message(msg.to_string());
    pb
}

/// 创建多进度条
pub fn create_multi_progress() -> MultiProgress {
    MultiProgress::new()
}

/// 确保目录存在
pub async fn ensure_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path).await?;
    }
    Ok(())
}

/// 提取 HTML 中的文本
pub async fn extract_text(html_path: &Path, selector: &str) -> Result<String> {
    let content = tokio::fs::read_to_string(html_path).await?;
    let document = scraper::Html::parse_document(&content);
    
    // 手动处理选择器错误
    let selector = scraper::Selector::parse(selector)
        .map_err(|e| anyhow::anyhow!(format!("选择器解析失败: {:?}", e)))?;
    
    document.select(&selector)
        .next()
        .map(|el| el.text().collect::<String>())
        .ok_or_else(|| anyhow::anyhow!("选择器未找到内容"))
}