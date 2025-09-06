use std::path::PathBuf;

use crate::core::get_index::get_index_from_stdin;
use crate::core::singlefile::Singlefile;
use crate::source::bilinovel::download::download_chapter_singlefile;
use crate::source::bilinovel::types::BiliNovel;
use crate::utils::browser::browser_server::{BrowserConfig, BrowserServer};
use crate::utils::config::DynamicConfig;
use crate::utils::epub::EpubGenerator;
use crate::utils::httpclient::types::RequestConfig;
use crate::utils::httpserver::{AppConfig, get_all_images, init_controller, start_server};
use crate::utils::terminal::clear_previous_line;
use anyhow::Result;
use crossterm::style::Stylize;
use regex::Regex;
use serde_json::json;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use crate::source::bilinovel::extract::{
    build_chapter, extract_author, extract_chapter, extract_description, extract_tags,
    extract_volume,
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
use tracing::{error, info};

use async_trait::async_trait; // 导入宏

///从链接获取书籍号
pub fn get_bilinovel(url: &str) -> Box<BiliNovel> {
   let re = match Regex::new("/novel/(\\d+)") {
       Ok(v) => v,
       Err(e) => {
        error!("{}",e);
        eprintln!("解析正则表达式错误，直接返回链接 {}",e);
        return Box::new(BiliNovel::new(url.to_string()));
       }
   };

   let id = re.captures(url)           
    .and_then(|ca| ca.get(1))          
    .map(|v| v.as_str())                
    .unwrap_or("");  
   if id.is_empty() {
       Box::new(BiliNovel::new(url.to_string()))
   }else {
        Box::new(BiliNovel::new(format!("https://www.linovelib.com/novel/{}.html",id)))
   }
   
}

#[async_trait]
impl Singlefile for BiliNovel {
    async fn display(&mut self) -> Result<()> {
        let config = RequestConfig::default();
        self.parser_book_http_async(config).await?;
        // self.parser_book_singlefile(dconfig).await?;
        println!("{}", &self);
        println!("--------------------");
        Ok(())
    }

    async fn download(&mut self) -> Result<()> {
        println!("\n");
        let index = loop {
            println!("输入你要下载的卷[支持空格分隔和 a-b 范围格式]:");
            let mut input = String::new();
            match std::io::stdin().read_line(&mut input) {
                Ok(_bytes_read) => {
                    let trimmed = input.trim().to_string();
                    if trimmed.is_empty() {
                        clear_previous_line(2).unwrap_or_else(|e| eprintln!("清除屏幕失败：{}", e));
                        continue;
                    }
                    match get_index_from_stdin(&trimmed) {
                        Ok(index) => break index,
                        Err(e) => {
                            clear_previous_line(3)
                                .unwrap_or_else(|e| eprintln!("清除屏幕失败：{}", e));
                            eprintln!("{}", e)
                        }
                    }
                }
                Err(e) => {
                    clear_previous_line(3).unwrap_or_else(|e| eprintln!("清除屏幕失败：{}", e));
                    // 读取失败，打印错误详情（而非终止程序）
                    eprintln!("读取输入出错：{}", e);
                }
            }
        };
        self.index = index.clone();

        //服务器的配置及启动
        let mut appconfig = AppConfig::default();
        appconfig.set_send_to_rust(true);
        appconfig.set_regex_pattern(r"^https://img3\.readpai\.com/.*");
        appconfig.set_open_download(true);

        init_controller(appconfig)?;
        start_server()?;

        //浏览器的配置及启动
        let mut browser = BrowserServer::new(BrowserConfig::load("./config/browser.json")?)?;
        browser.start().await?;
        let browser_server_url = browser.get_server_url();

        //解析小说页面的配置
        let mut config = DynamicConfig::new();
        config.load(PathBuf::from("./config/novel.json"))?;
        config.with_set("browser-server", json!(browser_server_url));

        //加载并发数、检查轮数等配置
        self.load_config("./config/bilinovel.json")?;

        //下载小说
        for i in index {
            if i < self.volume.len() as u8 {
                if let Some(v) = self.volume.get_mut(i as usize) {
                    println!("\n");
                    match v
                        .parser_by_singlefile(
                            config.clone(),
                            self.config
                                .get("max-concurrent")
                                .and_then(|val| val.as_number()) // 非数字→None
                                .and_then(|num| num.as_u64()) // 负数/小数→None
                                .and_then(|u64_val| u64_val.try_into().ok()) // 超usize范围→None
                                .unwrap_or(5),
                            &browser_server_url,
                        )
                        .await
                    {
                        Ok(_) => info!("{}下载成功", v.name),
                        Err(e) => {
                            error!("{}下载出错：{}", v.name, e);
                            eprintln!("{}下载出错：{}", v.name, e);
                            continue;
                        }
                    }

                    let max_echo = self
                        .config
                        .get("check-rounds")
                        .and_then(|val| val.as_number()) // 非数字→None
                        .and_then(|num| num.as_u64()) // 负数/小数→None
                        .and_then(|u64_val| u64_val.try_into().ok()) // 超usize范围→None
                        .unwrap_or(100);

                    for i in 1..=max_echo {
                        println!(
                            "开始检查章节[没有问题将提前退出]：第{}/{}轮检查……",
                            i, max_echo
                        );
                        match v.check_images(&browser_server_url).await {
                            Ok(_) => {
                                println!("检查章节完成！");
                                break;
                            }
                            Err(e) => {
                                clear_previous_line(1)
                                    .unwrap_or_else(|e| eprintln!("清除屏幕失败：{}", e));
                                eprintln!("检查出错 {}", e)
                            }
                        }
                    }

                    println!("打包[{}]中...", v.name.to_owned().dark_green());
                    let images = match get_all_images() {
                        Ok(v) => v,
                        Err(e) => {
                            error!("{}", e);
                            eprintln!("提取图片错误：{}", e);
                            continue;
                        }
                    };

                    let generator = EpubGenerator::new(&self.volume[i as usize], &images);
                    let file = match std::fs::File::create(format!(
                        "./output/{}.epub",
                        &self.volume[i as usize].name
                    )) {
                        Ok(v) => v,
                        Err(e) => {
                            error!("{}", e);
                            eprintln!("{}", e);
                            continue;
                        }
                    };
                    generator.generate_epub(file).unwrap_or_else(|e| eprintln!("打包章节出错：{}",e));
                }
            }
        }
        Ok(())
    }

    // fn get_novel(&self) -> Result<()> {
    //     let images = get_all_images()?;
    //     for i in self.index.to_owned() {
    //         if i < self.volume.len() as u8 {
    //             println!("打包中……");
    //             let generator = EpubGenerator::new(&self.volume[i as usize], &images);
    //             let file =
    //                 std::fs::File::create(format!("{}.epub", &self.volume[i as usize].name))?;
    //             generator.generate_epub(file)?;
    //         }
    //     }

    //     Ok(())
    // }
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

    pub fn load_config(&mut self, config_path: &str) -> Result<()> {
        self.config.load(PathBuf::from(config_path))?;
        Ok(())
    }
}

impl Novel {
    ///解析每个章节，需给定配置文件地址、浏览器地址、最大并发数
    pub async fn parser_by_singlefile(
        &mut self,
        config: DynamicConfig,    //配置
        max_concurrent: usize,    //最大并发数
        browser_server_url: &str, //浏览器地址
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

        // 创建进度监控器
        let progress = ProgressMonitor::new(self.chapters.len(), &self.name);

        // 设置并发量
        let semaphore = Arc::new(Semaphore::new(max_concurrent));

        // 创建临时文件池
        let temp_files = Arc::new(Mutex::new(VecDeque::new()));

        // 预先创建临时文件
        {
            let mut files_lock = temp_files.lock().await;
            for _ in 0..max_concurrent {
                let temp_file = NamedTempFile::new_in("./temp")?;
                files_lock.push_back(temp_file);
            }
        }

        let mut chapter_futures = vec![];

        // 使用索引而不是直接使用引用
        for (i, chapter) in self.chapters.iter().enumerate() {
            let semaphore = Arc::clone(&semaphore);

            let temp_files = Arc::clone(&temp_files);

            let browser_url = browser_server_url.to_string();
            let chapter_url = chapter.url.clone();
            let chapter_title = chapter.title.clone();
            let progress = progress.clone(); // 克隆进度监控器

            chapter_futures.push(async move {
                let permit = semaphore.acquire().await?;

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

                let result = new_chapter
                    .parser_by_singlefile(&browser_url, &crawl_path)
                    .await;

                // 更新进度
                match &result {
                    Ok(_) => progress.increment(),
                    Err(_) => {
                        progress.record_error();
                        // eprintln!("章节下载失败: {} - {}", chapter_title, e);
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

    ///检查每个章节的图片是否下载完成，需给定浏览器地址
    pub async fn check_images(&mut self, browser_server_url: &str) -> Result<()> {
        let mut images: HashMap<String, Vec<String>> = HashMap::new();
        if let Some(Some(_)) = get_image_by_url(&self.cover).ok() {
        } else {
            //解析小说页面的配置
            let mut config = DynamicConfig::new();
            config.load(PathBuf::from("./config/novel.json"))?;
            config.with_set("browser-server", json!(browser_server_url));
            let _ = download_from_url(&self.url, config).await;
        }

        for chapter in &self.chapters {
            for i in &chapter.image {
                if let Some(Some(_)) = get_image_by_url(i).ok() {
                    continue;
                } else {
                    images
                        .entry(chapter.url.to_owned())
                        .or_insert(Vec::new())
                        .push(i.to_owned());
                }
            }
        }
        // 创建临时文件（自动唯一命名和清理）
        let temp_file = NamedTempFile::new_in("./temp")?;
        let crawl_path = temp_file.path().to_str().unwrap_or("./temp/images.json");
        for (url, _) in images {
            download_chapter_singlefile(
                &url,
                browser_server_url,
                crawl_path,
                "./config/images.json",
            )
            .await?;
        }

        Ok(())
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
