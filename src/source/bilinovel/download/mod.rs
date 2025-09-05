use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use crate::source::bilinovel::extract::{build_chapter, extract_author, extract_chapter, extract_description, extract_tags, extract_volume};
use crate::source::bilinovel::types::{BiliNovel, Chapter, Novel};
use crate::utils::config::DynamicConfig;
use crate::utils::download::downl::down::download_from_url;
use crate::utils::httpclient::http_async::AsyncHttpClient;
use crate::utils::httpclient::types::RequestConfig;
use crate::utils::httpserver::get_image_by_url;
use crate::utils::progressbar::progress_monitor::ProgressMonitor;

use anyhow::{Context, Result, anyhow};
use futures::{StreamExt, stream};
use regex::Regex;
use scraper::{Html, Selector};
use serde_json::{Value, json};
use tokio::sync::{Mutex, Semaphore};
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

        let book_name: String = html
            .select(&book_name_selector)
            .next()
            .map(|e| e.text().collect())
            .unwrap_or_default();

        self.book_name = book_name;
        self.author = extract_author(html_content, "div.au-name")?;
        (self.nums, self.notice, self.description) =
            extract_description(html_content, "div.book-info")?;
        self.tags = Some(extract_tags(html_content, "div.book-label")?);
        self.volume = extract_volume(html_content, "div.book-vol-chapter")?;

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

        let book_name: String = html
            .select(&book_name_selector)
            .next()
            .map(|e| e.text().collect())
            .unwrap_or_default();

        self.book_name = book_name;
        self.author = extract_author(&book_response.body, "div.au-name")?;
        (self.nums, self.notice, self.description) =
            extract_description(&book_response.body, "div.book-info")?;
        self.tags = Some(extract_tags(&book_response.body, "div.book-label")?);
        self.volume = extract_volume(&book_response.body, "div.book-vol-chapter")?;

        Ok(())
    }
}

impl Novel {
    pub async fn parser_by_singlefile(
        &mut self,
        config: DynamicConfig,    //配置
        max_concurrent: usize,    //最大并发数
        browser_server_url: &str, //浏览器地址
        crawl_path: &str,         //爬虫会话文件
        script_path: &str,        //脚本地址-用于下载
    ) -> Result<()> {
        let novel_vec: Vec<Value> = download_from_url(&self.url, config).await?;
        let novel_json = novel_vec
            .first()
            .with_context(|| format!("没有从{}得到任何内容", self.url))
            .map_err(|e| {
                error!("没有从{}得到任何内容", self.url);
                anyhow!("{}", e)
            })?;

        let html_content = novel_json["content"]
            .as_str()
            .with_context(|| "缺少content字段")?;

        let html = Html::parse_document(html_content);
        self.author = extract_author(html_content, "div.au-name")?;
        (_, _, self.description) = extract_description(html_content, "div.book-info")?;
        self.tags = Some(extract_tags(html_content, "div.book-label")?);
        let cover_selector = Selector::parse("div.book-img>img").map_err(|e| {
            error!("{}", e);
            anyhow!("{e}")
        })?;
        self.cover = html
            .select(&cover_selector)
            .next()
            .map(|e| e.attr("data-original-src").unwrap_or_default())
            .unwrap_or_default()
            .to_string();
        self.chapters = build_chapter(html_content, "div.book-new-chapter")?;
        // println!("即将并发处理章节");
        
        // 创建进度监控器
        let progress = ProgressMonitor::new(self.chapters.len(), &self.name);

        // 在这运行并行任务
        // 并发处理章节
        let semaphore = Arc::new(Semaphore::new(max_concurrent));

        // 2. 可用序号池：初始填充1-3，用Mutex保护队列操作
        let available_ids = Arc::new(Mutex::new({
            let mut ids = VecDeque::new();
            for i in 1..=max_concurrent {
                ids.push_back(i);
            }
            ids
        }));

        let mut chapter_futures = vec![];

        // 使用索引而不是直接使用引用
        for (i, chapter) in self.chapters.iter().enumerate() {
            let semaphore = Arc::clone(&semaphore);
            let ids = Arc::clone(&available_ids);
            let browser_url = browser_server_url.to_string();
            let crawl_path = crawl_path.to_string(); // 不需要修改路径
            let script_path = script_path.to_string();
            let chapter_url = chapter.url.clone();
            let chapter_title = chapter.title.clone();
            let progress = progress.clone(); // 克隆进度监控器

            chapter_futures.push(async move {
                let permit = semaphore.acquire().await?;

                let mut ids_lock = ids.lock().await;
                let permit_id = ids_lock.pop_front().unwrap(); // 弹出第一个可用序号
                drop(ids_lock); // 尽早释放锁

                let crawl_path = format!("{}_{}.json", crawl_path, permit_id);
                let mut new_chapter = Chapter {
                    url: chapter_url,
                    title: chapter_title.clone(),
                    context: Vec::new(),
                    image: Vec::new(),
                };

                let result = new_chapter
                    .parser_by_singlefile(&browser_url, &crawl_path, &script_path)
                    .await;
                
                // 更新进度
                // progress.increment();
                match &result {
                    Ok(_) => progress.increment(),
                    Err(e) => {
                        progress.record_error();
                        eprintln!("章节下载失败: {} - {}", chapter_title, e);
                    }
                }

                // 释放阶段：将序号放回可用池
                let mut ids_lock = ids.lock().await;
                ids_lock.push_back(permit_id); // 序号回收
                drop(ids_lock);

                drop(permit);
                Ok((i, result, new_chapter))
            });
        }

        // 等待所有任务完成
        let results: Vec<Result<(usize, Result<()>, Chapter)>> = stream::iter(chapter_futures)
            .buffer_unordered(max_concurrent)
            .collect()
            .await;

        // 完成进度监控
        progress.finish();

        // 更新原始章节数据并检查错误
        for result in results {
            match result {
                Ok((i, chapter_result, updated_chapter)) => {
                    if let Some(chapter) = self.chapters.get_mut(i) {
                        *chapter = updated_chapter;
                        chapter_result?;
                    }
                }
                Err(e) => {
                    eprintln!("任务执行失败: {}", e);
                }
            }
        }
        Ok(())
    }

    pub async fn check_images(&mut self, browser_server_url:&str, crawl_path:&str, script_path:&str) -> Result<()> {
        let mut images: HashMap<String, Vec<String>> = HashMap::new();
        for chapter in &self.chapters {
            for i in &chapter.image {
                if let Some(Some(_)) = get_image_by_url(i).ok() {
                    continue;
                }else {
                    images.entry(chapter.url.to_owned()).or_insert(Vec::new()).push(i.to_owned());
                }
            }
        }

        for (url, _) in images {
            download_chapter_singlefile(&url, browser_server_url, crawl_path, script_path).await?;
        }

        Ok(())
    }
}

impl Chapter {
    pub async fn parser_by_singlefile(
        &mut self,
        browser_server_url: &str, //浏览器地址
        crawl_path: &str,         //
        script_path: &str,
    ) -> Result<()> {
        let htmls =
            download_chapter_singlefile(&self.url, browser_server_url, crawl_path, script_path)
                .await?;
        let mut src_vec = Vec::new();
        let mut context = Vec::new();
        for html in htmls {
            let url = html
                .pointer("/request/url")
                .and_then(|v| v.as_str())
                .unwrap_or("url获取失败");
            let (img_src, content) = extract_chapter(
                html["content"].as_str().unwrap_or_default(),
                "div#TextContent",
                "data-original-src",
                vec!["div.dag","center#show-more-images","div.google-auto-placed","div.ap_container"]
            )
            .unwrap_or((
                vec![format!("章节解析出错 url:{}", url)],
                format!("章节解析出错 url:{}", url),
            ));
            src_vec.extend(img_src);
            context.push(content);
        }
        self.context = context;
        self.image = src_vec;
        Ok(())
    }
}

pub async fn download_chapter_singlefile(
    url: &str,
    browser_server_url: &str, //浏览器地址
    crawl_path: &str,         //爬虫会话文件
    script_path: &str,        //用户脚本路径
) -> Result<Vec<Value>> {
    let mut config = DynamicConfig::new();
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
    config.with_set("remove-hidden-elements", json!(true))
          .with_set("browser-server", json!(browser_server_url))
          .with_set("dump-content", json!(true))
          .with_set("output-json", json!(true))
          .with_set("block-images", json!(true))
          .with_set("crawl-links", json!(true))
          .with_set("crawl-no-parent", json!(true))
          .with_set("crawl-inner-links-only", json!(true))
        //   .with_set("console-messages-file", json!("log.json"))
          .with_set("crawl-max-depth", json!(20))
          .with_set("crawl-sync-session", json!(crawl_path))
          .with_set("browser-script", json!(script_path))
          .with_set("browser-wait-delay", json!(1000))
          .with_set("browser-load-max-time", json!(240000))
          .with_set("browser-capture-max-time", json!(240000))
          .with_set("crawl-rewrite-rule", json!([
                "^https://www\\.linovelib\\.com/novel/\\d+/vol_.*\\.html$ https://www.linovelib.com/",
                format!("^https://www\\.linovelib\\.com/novel/(\\d+)/\\d+(_\\d+)?\\.html$ https://www.linovelib.com/novel/$1/{}$2.html",chapter_id),
                "^https://www\\.linovelib\\.com/novel/\\d+/catalog.*$ https://www.linovelib.com/"]));

    download_from_url(url, config).await
}
