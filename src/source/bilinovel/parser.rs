use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::source::bilinovel::download::download_chapter_singlefile;
use crate::source::bilinovel::types::BiliNovel;
use crate::utils::config::DynamicConfig;
use crate::utils::httpclient::types::RequestConfig;
use crate::utils::httpserver::{save_images_to_file, update_config};
use crate::utils::input::{UserCommand, create_key_listener};
use anyhow::Result;
use regex::Regex;
use serde_json::json;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use crate::source::bilinovel::extract::{
    build_chapter, extract_author, extract_chapter, extract_description, extract_tags,
    extract_volume, extract_volume_catalog,
};
use crate::source::bilinovel::types::{Chapter, Novel};
use crate::utils::download::downl::down::download_from_url;
use crate::utils::httpclient::http_async::AsyncHttpClient;
use crate::utils::httpserver::get_image_by_url;
use crate::utils::progressbar::progress_monitor::ProgressMonitor;

use anyhow::{Context, anyhow};
use futures::{StreamExt, stream};
use scraper::{Html, Selector};
use serde_json::Value;
use tempfile::NamedTempFile;
use tokio::sync::{Mutex, Semaphore};
use tracing::{error, info, warn};

///从链接获取书籍号
pub fn get_bilinovel(url: &str) -> Box<BiliNovel> {
    let re = match Regex::new("/novel/(\\d+)") {
        Ok(v) => v,
        Err(e) => {
            error!("{}", e);
            eprintln!("解析正则表达式错误，直接返回链接 {}", e);
            return Box::new(BiliNovel::new(url.to_string(), String::new()));
        }
    };

    let id = re
        .captures(url)
        .and_then(|ca| ca.get(1))
        .map(|v| v.as_str())
        .unwrap_or("");
    if id.is_empty() {
        error!("没有提取到书籍id,直接返回链接：{}", url);
        Box::new(BiliNovel::new(url.to_string(), String::new()))
    } else {
        info!("正确提取到书籍id:{}", id);
        Box::new(BiliNovel::new(
            format!("https://www.linovelib.com/novel/{}.html", id),
            format!("https://www.linovelib.com/novel/{}/catalog", id),
        ))
    }
}

///检查是否有缺页
pub fn has_empty_string(vec: &Vec<String>) -> bool {
    vec.is_empty() || vec.iter().any(|s| s.is_empty())
}

impl BiliNovel {
    ///通过single-file解析页面
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

    ///通过rust服务器解析页面
    pub async fn parser_book_http_async(&mut self, config: RequestConfig) -> Result<()> {
        let client = AsyncHttpClient::new(config).map_err(|e| {
            error!("创建客户端失败: {}", e);
            anyhow::anyhow!("创建客户端失败: {}", e)
        })?;
        let book_response = client.get(&self.url).await.map_err(|e| {
            error!("发送请求失败：{}", e);
            anyhow!("发送请求失败：{}", e)
        })?;

        if !book_response.is_success() {
            error!(
                "获取失败: url:{},\tstatus:{}",
                book_response.url, book_response.status
            );
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
        if self.volume.is_empty() {
            let catalog = client.get(&self.catalog).await.map_err(|e| {
                error!("发送请求失败：{}", e);
                anyhow!("发送请求失败：{}", e)
            })?;

            if !catalog.is_success() {
                error!(
                    "获取失败: url:{},\tstatus:{}",
                    book_response.url, book_response.status
                );
                return Err(anyhow!(
                    "获取失败: url:{},\tstatus:{}",
                    book_response.url,
                    book_response.status
                ));
            };
            self.volume = extract_volume_catalog(&catalog.body, "div#volume-list")?;
        }

        Ok(())
    }

    pub fn load_config(&mut self, config_path: &str) -> Result<()> {
        info!("开始从{}加载bilinovel的配置", config_path);
        self.config = super::types::NovelConfig::load(PathBuf::from(config_path))?;
        Ok(())
    }
}

impl Novel {
    ///解析每个章节，需给定配置文件地址、浏览器地址、最大并发数
    pub async fn parser_by_singlefile(
        &mut self,
        config: DynamicConfig,                   //配置
        max_concurrent: usize,                   //最大并发数
        browser_server_url: &str,                //浏览器地址
        novel_download_state: Arc<Mutex<Novel>>, // 修改为 Arc<Mutex<Novel>>
        state_path: &str,                        // 状态文件路径
        images_path: &str,                       //图片保存地址
        compression_level: u32,                  //图片压缩等级
        save_interval: usize,
    ) -> Result<()> {
        let retry_novel = 5;
        //不为空说明是恢复的下载
        if self.chapters.is_empty() {
            info!("chapter为空");
            let mut html_content = String::new();
            for _ in 0..retry_novel {
                let novel_vec: Vec<Value> = download_from_url(&self.url, config.clone()).await?;
                let novel_json = novel_vec
                    .first()
                    .with_context(|| format!("没有从{}得到任何内容", self.url))
                    .map_err(|e| {
                        error!("没有从{}得到任何内容", self.url);
                        anyhow!("{}", e)
                    })?;

                html_content = novel_json["content"]
                    .as_str()
                    .with_context(|| "缺少content字段")?
                    .to_owned();
                if html_content.is_empty()
                    || html_content.contains("Cloudflare to restrict access")
                    || html_content.contains("503 Service Temporarily Unavailable")
                {
                    continue;
                } else {
                    break;
                }
            }

            let html = Html::parse_document(&html_content);
            self.author = extract_author(&html_content, "div.au-name")?;
            (_, _, self.description) = extract_description(&html_content, "div.book-info")?;
            self.tags = Some(extract_tags(&html_content, "div.book-label")?);
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

            self.chapters = build_chapter(&html_content, "div.book-new-chapter")?;
        }

        //为空说明是新建的下载，因为恢复的下载但是pending_chapter_indices为空的情况以及排除
        if novel_download_state
            .lock()
            .await
            .pending_chapter_indices
            .is_empty()
        {
            info!("新建的下载");
            self.pending_chapter_indices = (0..self.chapters.len()).collect();
            *novel_download_state.lock().await = self.clone();
        }

        // 获取待下载章节数量
        let pending_count = self.pending_chapter_indices.len();

        // 创建进度监控器
        let progress = ProgressMonitor::new(pending_count, &self.name);

        //返回较小的那一个
        let max_concurrent = self.chapters.len().min(max_concurrent);

        // 设置并发量
        let semaphore = Arc::new(Semaphore::new(max_concurrent));

        // 创建临时文件池
        let temp_files = Arc::new(Mutex::new(VecDeque::new()));

        // 预先创建临时文件
        {
            info!("创建临时文件用来并发下载章节");
            let mut files_lock = temp_files.lock().await;
            for _ in 0..max_concurrent {
                let temp_file = NamedTempFile::new_in("./temp/temp")?;
                files_lock.push_back(temp_file);
            }
        }

        if self.chapters.is_empty() {
            return Err(anyhow!("chapter为空"));
        }

        info!("chapter数量：{}", self.chapters.len());
        info!("待下载数量：{}", self.pending_chapter_indices.len());

        let mut chapter_futures = vec![];
        let completed_count = Arc::new(AtomicUsize::new(0));
        // let save_interval = max_concurrent; // 每完成max章保存一次

        // 创建按键监听器
        let (stop_tx, mut cmd_rx) = create_key_listener();

        // 创建取消标志
        let cancelled = Arc::new(AtomicBool::new(false));
        let cancelled_clone = cancelled.clone();

        // 启动一个任务来监听按键事件
        let key_listener_handle = tokio::spawn(async move {
            while let Some(cmd) = cmd_rx.recv().await {
                match cmd {
                    UserCommand::Quit => {
                        println!("\n用户请求退出");
                        cancelled_clone.store(true, Ordering::Relaxed);
                        break;
                    }
                    _ => {}
                }
            }
        });

        // 使用索引而不是直接使用引用
        for (i, chapter) in self.chapters.iter().enumerate() {
            // 检查是否取消
            if cancelled.load(Ordering::Relaxed) {
                println!("下载已被取消");
                break;
            }

            // 检查是否在待下载列表中
            if !self.pending_chapter_indices.contains(&i) || self.pending_chapter_indices.is_empty()
            {
                continue;
            }

            let semaphore = Arc::clone(&semaphore);
            let temp_files = Arc::clone(&temp_files);
            let browser_url = browser_server_url.to_string();
            let chapter_url = chapter.url.clone();
            let chapter_title = chapter.title.clone();
            let progress = progress.clone();
            let completed_count = Arc::clone(&completed_count);
            let cancelled_clone = cancelled.clone();

            // 克隆 Arc 用于在异步任务中使用
            let novel_download_state = Arc::clone(&novel_download_state);
            let state_path = state_path.to_string();

            chapter_futures.push(async move {
                let permit = semaphore.acquire().await?;
                // 检查是否取消
                if cancelled_clone.load(Ordering::Relaxed) {
                    return Ok((i, Err(anyhow::anyhow!("任务被取消")), Chapter::default()));
                }

                // 从池中获取临时文件
                let mut files_lock = temp_files.lock().await;
                let temp_file = files_lock.pop_front().unwrap();
                drop(files_lock);

                // 获取临时文件路径
                let crawl_path = temp_file.path().to_str().unwrap().to_string();

                let mut new_chapter = Chapter {
                    url: chapter_url,
                    title: chapter_title.clone(),
                    context: Vec::new(),
                    image: Vec::new(),
                };

                info!("下载第{}个章节中", i);

                //等待一段时间
                tokio::time::sleep(std::time::Duration::from_secs((i % max_concurrent) as u64))
                    .await;
                tokio::time::sleep(std::time::Duration::from_millis(
                    (i % max_concurrent) as u64,
                ))
                .await;

                let result = new_chapter
                    .parser_by_singlefile(&browser_url, &crawl_path)
                    .await;

                // 更新进度
                match &result {
                    Ok(_) => {
                        info!("章节下载成功：{}", chapter_title);
                        progress.increment();

                        // 更新 NovelDownloadState
                        let mut state = novel_download_state.lock().await;

                        // 更新章节内容
                        if let Some(existing_chapter) = state.chapters.get_mut(i) {
                            *existing_chapter = new_chapter.clone();
                        }

                        // 从待下载列表中移除
                        if let Some(pos) = state
                            .pending_chapter_indices
                            .iter()
                            .position(|&idx| idx == i)
                        {
                            state.pending_chapter_indices.remove(pos);
                        }

                        let current_count = completed_count.fetch_add(1, Ordering::SeqCst) + 1;

                        // 定期保存或最后一个下载保存
                        if current_count % save_interval == 0 || current_count == pending_count {
                            if let Ok(content) = serde_json::to_string_pretty(&*state) {
                                match std::fs::write(&state_path, content) {
                                    Ok(_) => {
                                        info!("成功保存状态文件");
                                    }
                                    Err(e) => {
                                        error!("保存状态文件失败：{}", e);
                                    }
                                }
                            }
                            match save_images_to_file(images_path, compression_level) {
                                Ok(_) => {
                                    info!("成功保存图片");
                                }
                                Err(e) => {
                                    error!("保存图片失败：{}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        progress.record_error();
                        error!("章节下载失败: {} - {}", chapter_title, e);
                    }
                }

                // 将临时文件放回池中
                let mut files_lock = temp_files.lock().await;
                files_lock.push_back(temp_file);
                drop(files_lock);

                drop(permit);
                Ok((i, result, new_chapter))
            });
        }
        // 等待所有任务完成
        let results: Vec<Result<(usize, Result<()>, Chapter)>> = stream::iter(chapter_futures)
            .buffer_unordered(max_concurrent)
            .collect()
            .await;
        info!("所有任务下载完成");
        // 完成进度监控
        progress.finish();

        let cancelled_clone = cancelled.clone();
        if cancelled_clone.load(Ordering::Relaxed) {
            println!("下载被取消，正在保存数据...");
            let state = novel_download_state.lock().await;
            if let Ok(content) = serde_json::to_string_pretty(&*state) {
                match std::fs::write(&state_path, content) {
                    Ok(_) => {
                        println!("成功保存状态文件:{}", state_path);
                        info!("成功保存状态文件");
                    }
                    Err(e) => {
                        eprintln!("保存状态文件失败：{}", e);
                        error!("保存状态文件失败：{}", e);
                    }
                }
            }
            // match save_images_to_json_file(images_path, false) {
            //     Ok(_) => {
            //         println!("成功保存图片:{}", images_path);
            //         info!("成功保存图片");
            //     }
            //     Err(e) => {
            //         eprintln!("保存图片失败：{}", e);
            //         error!("保存图片失败：{}", e);
            //     }
            // }
            match save_images_to_file(images_path, compression_level) {
                Ok(_) => {
                    println!("成功保存图片:{}", images_path);
                    info!("成功保存图片");
                }
                Err(e) => {
                    eprintln!("保存图片失败：{}", e);
                    error!("保存图片失败：{}", e);
                }
            }
        }

        // 停止按键监听器
        let _ = stop_tx.send(()).await;
        key_listener_handle.await?;

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
                    error!("任务执行失败: {}", e);
                    eprintln!("任务执行失败: {}", e);
                }
            }
        }

        Ok(())
    }

    ///检查每个章节的图片和文字是否下载完成，需给定浏览器地址
    pub async fn check(
        &mut self,
        config: DynamicConfig,
        browser_server_url: &str,
        check_concurrent: usize,
        novel_download_state: Arc<Mutex<Novel>>,
        state_path: &str,
        images_path: &str,
        compression_level: u32,
        save_interval: usize,
    ) -> Result<()> {
        info!("开始检查是否缺少图片或缺页");

        let mut url_map: HashMap<String, Vec<String>> = HashMap::new();
        let mut index = HashSet::new();
        let mut res = Ok(());
        if let Ok(Some(_)) = get_image_by_url(&self.cover) {
        } else {
            warn!(
                "缺少封面图片。小说url：{}，图片url：{}",
                self.url, self.cover
            );
            warn!("开始重写下载小说url：{}，图片url：{}", self.url, self.cover);
            res = Err(anyhow!("缺少封面图片"));
            //解析小说页面的配置
            let mut config = DynamicConfig::new();
            config.load(PathBuf::from("./config/novel.json"))?;
            config.with_set("browser-server", json!(browser_server_url));
            download_from_url(&self.url, config).await?;
        }

        for (i, chapter) in self.chapters.iter().enumerate() {
            if has_empty_string(&chapter.context) {
                index.insert(i);
                url_map.entry(chapter.url.to_owned()).or_insert(Vec::new());
                warn!("缺页。章节url：{}", chapter.url);
            }
            for image_url in &chapter.image {
                if let Some(Some(_)) = get_image_by_url(image_url).ok() {
                    continue;
                } else {
                    warn!("缺少图片。章节url：{}，图片url：{}", chapter.url, image_url);

                    update_config(|f| {
                        f.regex_pattern.push(image_url.to_owned());
                    })
                    .unwrap_or_else(|e| error!("更新正则表达式失败：{}", e));

                    index.insert(i);
                    url_map
                        .entry(chapter.url.to_owned())
                        .or_insert(Vec::new())
                        .push(image_url.to_owned());
                }
            }
        }
        // 创建临时文件（自动唯一命名和清理）
        if !index.is_empty() {
            res = Err(anyhow!("存在章节缺少图片或缺页"));

            //将有问题的页面重新假如待下载行列
            self.pending_chapter_indices = Vec::from_iter(index);
            novel_download_state.lock().await.pending_chapter_indices =
                self.pending_chapter_indices.clone();

            match self
                .parser_by_singlefile(
                    config.clone(),
                    check_concurrent,
                    &browser_server_url,
                    Arc::clone(&novel_download_state), // 传入 Arc<Mutex<Novel>>
                    state_path,                        // 传入状态文件路径
                    images_path,
                    compression_level,
                    save_interval,
                )
                .await
            {
                Ok(_) => {
                    info!("重新下载成功");
                }
                Err(_) => {
                    error!("重新下载失败");
                }
            }
        }

        res
    }
}

impl Chapter {
    ///解析章节内容
    pub async fn parser_by_singlefile(
        &mut self,
        browser_server_url: &str, //浏览器地址
        crawl_path: &str,         //爬虫会话文件
    ) -> Result<()> {
        let htmls = download_chapter_singlefile(
            &self.url,
            browser_server_url,
            crawl_path,
            r"config\chapter.json",
        )
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
                vec![
                    "div.dag",
                    "center#show-more-images",
                    "div.google-auto-placed",
                    "div.ap_container",
                ],
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
