use std::path::PathBuf;
use crate::utils::config::DynamicConfig;
use crate::utils::download::downl::down::download_from_url;
use anyhow::{Result, anyhow};
use regex::Regex;
use serde_json::{Value, json};


///根据给定的第一页的url下载章节页面，还需给定浏览器地址、爬虫会话文件地址、配置文件地址
pub async fn download_chapter_singlefile(
    url: &str,
    browser_server_url: &str, //浏览器地址
    crawl_path: &str,         //爬虫会话文件
    config_path: &str,
) -> Result<Vec<Value>> {
    let mut config = DynamicConfig::new();
    config.load(PathBuf::from(config_path))?;

    let re = Regex::new(r"novel/\d+/(\d{2,})")?;
    let chapter_id = re
        .captures(url)
        .ok_or(anyhow!(
            "URL 格式不符合预期，未找到 novel 后的数字 url:{}",
            url
        ))?
        .get(1)
        .ok_or(anyhow!("未捕获到 novel 后的数字 url:{}", url))?
        .as_str();

    config.with_set("browser-server", json!(browser_server_url))
          .with_set("crawl-sync-session", json!(crawl_path))
          .with_set("crawl-rewrite-rule", json!([
                "^https://www\\.linovelib\\.com/novel/\\d+/vol_.*\\.html$ https://www.linovelib.com/",
                format!("^https://www\\.linovelib\\.com/novel/(\\d+)/\\d+(_\\d+)?\\.html$ https://www.linovelib.com/novel/$1/{}$2.html",chapter_id),
                "^https://www\\.linovelib\\.com/novel/\\d+/catalog.*$ https://www.linovelib.com/"]));

    download_from_url(url, config).await
}
