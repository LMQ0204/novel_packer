use crate::source::bilinovel::parser::{extract_description, extract_tags, extract_volume};
use crate::source::bilinovel::types::{BiliNovel, Chapter, Novel, Tags};
use crate::utils::config::DynamicConfig;
use crate::utils::download::downl::down::download_from_url;
use crate::utils::httpclient::http_async::AsyncHttpClient;
use crate::utils::httpclient::types::RequestConfig;
use anyhow::{Context, Result, anyhow};
use html2text::config;
use regex::Regex;
use scraper::{Html, Selector};
use serde_json::{Value, json};
use tracing::error;

impl BiliNovel {
    pub async fn parser_book_singlefile(&mut self, config: DynamicConfig) -> Result<()> {
        let book_vec: Vec<Value> = download_from_url(&self.url, config).await?;
        let book_json = book_vec
            .first()
            .with_context(|| format!("没有从{}得到任何内容", self.url))
            .map_err(|e| {
                error!("没有从{}得到任何内容", self.url);
                anyhow!("{}", e)
            })?;

        // 提取书籍信息时，一个链接应该只能得到一个Value。除非设置了一些奇怪的配置项
        // assert_eq!(1, book_json.len());

        let html_content = book_json["content"]
            .as_str()
            .with_context(|| "缺少content字段")?;

        let html = Html::parse_document(html_content);

        let book_name_selector = Selector::parse("div.book-info>h1.book-name").map_err(|e| {
            error!("{}", e);
            anyhow!("{e}")
        })?;

        let description_selector = Selector::parse("div.book-info").map_err(|e| {
            error!("{}", e);
            anyhow!("{e}")
        })?;

        let tags_selector = Selector::parse("div.book-label").map_err(|e| {
            error!("{}", e);
            anyhow!("{e}")
        })?;

        let volume_selector = Selector::parse("div.book-vol-chapter").map_err(|e| {
            error!("{}", e);
            anyhow!("{e}")
        })?;

        let book_name: String = html
            .select(&book_name_selector)
            .next()
            .map(|e| e.text().collect())
            .unwrap_or_default();

        self.book_name = book_name;
        (self.nums, self.notice, self.description) =
            extract_description(&html, description_selector)?;
        self.tags = Some(extract_tags(&html, tags_selector)?);
        self.volume = extract_volume(&html, volume_selector)?;

        Ok(())
    }

    pub async fn parser_book_http_async(&mut self, config: RequestConfig) -> Result<()> {
        let client =
            AsyncHttpClient::new(config).map_err(|e| anyhow::anyhow!("创建客户端失败: {}", e))?;
        let book_response = client
            .get(&self.url)
            .await
            .map_err(|e| anyhow!("发送请求失败：{}", e))?;

        if !book_response.is_success() {
            return Err(anyhow!(
                "获取失败: url:{},\tstatus:{}",
                book_response.url,
                book_response.status
            ));
        };
        let html = Html::parse_document(&book_response.body);

        let book_name_selector = Selector::parse("div.book-info>h1.book-name").map_err(|e| {
            error!("{}", e);
            anyhow!("{e}")
        })?;

        let description_selector = Selector::parse("div.book-info").map_err(|e| {
            error!("{}", e);
            anyhow!("{e}")
        })?;

        let tags_selector = Selector::parse("div.book-label").map_err(|e| {
            error!("{}", e);
            anyhow!("{e}")
        })?;

        let volume_selector = Selector::parse("div.book-vol-chapter").map_err(|e| {
            error!("{}", e);
            anyhow!("{e}")
        })?;

        let book_name: String = html
            .select(&book_name_selector)
            .next()
            .map(|e| e.text().collect())
            .unwrap_or_default();

        self.book_name = book_name;
        (self.nums, self.notice, self.description) =
            extract_description(&html, description_selector)?;
        self.tags = Some(extract_tags(&html, tags_selector)?);
        self.volume = extract_volume(&html, volume_selector)?;

        Ok(())
    }

    pub async fn download_novel_singlefile(&mut self, inedx: Vec<u8>) -> Result<()> {
        Ok(())
    }
}

impl Novel {
    pub async fn download_by_singlefile(&mut self, config: DynamicConfig) -> Result<()> {
        Ok(())
    }
}

impl Chapter {
    pub async fn parser_chapter_singlefile(&mut self, config: DynamicConfig) -> Result<()> {
        Ok(())
    }
}

pub async fn download_by_singlefile(
    url: &str,
    browser_server_url: &str,
    crawl_path: &str,
    script_path: &str,
) -> Result<Vec<Value>> {
    let mut config = DynamicConfig::new();
    let re = Regex::new("(\\d{2,})")?;
    let chapter_id = re
        .captures(url)
        .ok_or(anyhow!("URL 格式不符合预期，未找到 novel 后的数字"))?
        .get(2)
        .ok_or(anyhow!("未捕获到 novel 后的数字"))?
        .as_str();
    
    config.with_set("remove-hidden-elements", json!(true))
          .with_set("browser-server", json!(browser_server_url))
          .with_set("dump-content", json!(true))
          .with_set("output-json", json!(true))
          .with_set("block-images", json!(true))
          .with_set("crawl-links", json!(true))
          .with_set("crawl-no-parent", json!(true))
          .with_set("crawl-inner-links-only", json!(true))
          .with_set("crawl-max-depth", json!(20))
          .with_set("crawl-sync-session", json!(crawl_path))
          .with_set("browser-script", json!(script_path))
          .with_set("crawl-rewrite-rule", json!([
                "^https://www\\.linovelib\\.com/novel/\\d+/vol_.*\\.html$ https://www.linovelib.com/",
                format!("^https://www\\.linovelib\\.com/novel/(\\d+)/\\d+(_\\d+)?\\.html$ https://www.linovelib.com/novel/$1/{}$2.html",chapter_id),
                "^https://www\\.linovelib\\.com/novel/\\d+/catalog.*$ https://www.linovelib.com/"
           ]));
    download_from_url(url, config).await
}
