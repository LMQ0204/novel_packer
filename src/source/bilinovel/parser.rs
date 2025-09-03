use crate::source::bilinovel::types::{BiliNovel, Novel, Tags};
use crate::utils::httpclient::types::RequestConfig;
use crate::{core::singlefile::Singlefile};
use anyhow::{anyhow,Result};
use scraper::{Html, Selector};
use tracing::error;
use url::Url;

use async_trait::async_trait; // 导入宏

#[async_trait]
impl Singlefile for BiliNovel {
    async fn display(&mut self) -> Result<()> {
        // let user_input = read_url_from_stdin();
        // let mut book = BiliNovel::new(user_input);
        // let config = DynamicConfig::default();
        // book.parser_book_singlefile(config).await?;
        let config = RequestConfig::default();
        self.parser_book_http_async(config).await?;
        println!("{}", &self);
        println!("--------------------");
        Ok(())
    }
    
    fn init(&mut self) -> Result<()> {
        Ok(())
    }
}


pub fn extract_description(html: &Html, selector: Selector) -> Result<(String, String, String)> {
    let nums_selector = Selector::parse("div.nums > span").map_err(|e| {
        error!("{}", e);
        anyhow!("{e}")
    })?;

    let des_selector = Selector::parse("div.book-dec>p").map_err(|e| {
        error!("{}", e);
        anyhow!("{e}")
    })?;

    let notice_selector = Selector::parse("div.notice").map_err(|e| {
        error!("{}", e);
        anyhow!("{e}")
    })?;

    let mut nums = String::new();
    let mut notice = String::new();
    let mut description = String::new();
    if let Some(element) = html.select(&selector).next() {
        let nums_txt = element
            .select(&nums_selector)
            .map(|span| {
                span.text()
                    .fold(String::new(), |mut s, t| {
                        s.push_str(t);
                        s
                    })
                    .trim()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\t");
        let des = element.select(&des_selector)
            .next()
                .map_or_else(
                    || String::new(),
                    |s| html2text::from_read(s.html().as_bytes(), 100).unwrap_or("提取失败".to_string()),
                )
                .trim()
                .to_string();
        let notice_txt = element.select(&notice_selector)
            .next()
                .map_or_else(
                    || String::new(),
                    |s| html2text::from_read(s.html().as_bytes(), 100).unwrap_or("提取失败".to_string()),
                )
                .trim()
                .to_string();

        // println!("nums:{nums}");
        nums.push_str(&nums_txt);
        description.push_str(&des);
        notice.push_str(&notice_txt);
    }

    Ok((nums, notice, description))
}

pub fn extract_tags(html: &Html, selector: Selector) -> Result<Tags> {
    let state_selector = Selector::parse("a.state").map_err(|e| {
        error!("{}", e);
        anyhow!("{e}")
    })?;
    let label_selector = Selector::parse("a.label").map_err(|e| {
        error!("{}", e);
        anyhow!("{e}")
    })?;
    let span_selector = Selector::parse("span>a").map_err(|e| {
        error!("{}", e);
        anyhow!("{e}")
    })?;
    let mut res = Tags::new();
    if let Some(element) = html.select(&selector).next() {
        res.state = element
            .select(&state_selector)
            .next()
            .map_or_else(|| String::new(), |s| s.text().collect::<String>());

        res.label = element
            .select(&label_selector)
            .map(|label| {
                label.text().fold(String::new(), |mut s, t| {
                    s.push_str(t);
                    s
                })
                .trim()
                .to_string()
            })
            .collect();

        res.span = element
            .select(&span_selector)
            .map(|span| {
                span.text().fold(String::new(), |mut s, t| {
                    s.push_str(t);
                    s
                })
                .trim()
                .to_string()
            })
            .collect()
    }
    Ok(res)
}

pub fn extract_volume(html: &Html, selector: Selector) -> Result<Vec<Novel>> {
    let volume_selector = Selector::parse("a").map_err(|e| {
        error!("{}", e);
        anyhow!("{e}")
    })?;
    let mut res = Vec::new();
    if let Some(element) = html.select(&selector).next() {
        res = element
            .select(&volume_selector)
            .filter_map(|e| {
                let mut url = e.value().attr("href").unwrap_or_default().to_string();
                if let Ok(full_url) = Url::parse(&url) {
                    url = full_url.to_string();
                } else if Url::parse(&format!("https://www.linovelib.com{}", url)).is_ok() {
                    url = format!("https://www.linovelib.com{}", url);
                }

                let name = e.value().attr("title").unwrap_or_default().to_string();
                Some(Novel::new(url, name))
            })
            .collect::<Vec<_>>();
        res.reverse();
    }
    Ok(res)
}
