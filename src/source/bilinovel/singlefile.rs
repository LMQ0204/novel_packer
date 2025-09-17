use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::get_index::get_index_from_stdin;
use crate::core::singlefile::Singlefile;
use crate::source::bilinovel::types::{BiliNovel, Novel};
use crate::utils::browser::browser_server::{BrowserConfig, BrowserServer};
use crate::utils::config::DynamicConfig;
use crate::utils::epub::EpubGenerator;
use crate::utils::httpclient::types::RequestConfig;
use crate::utils::httpserver::{
    AppConfig, get_all_images, init_controller, load_images_from_file, start_server, update_config,
};
use crate::utils::input::{UserCommand, create_key_listener};
use crate::utils::terminal::clear_previous_line;
use anyhow::Result;
use async_trait::async_trait;
use crossterm::style::Stylize;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info}; // 导入宏

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
        let appconfig = AppConfig::from_file("./config/http.json").unwrap_or_else(|e| {
            error!("服务器配置读取失败，将使用默认值：{}", e);
            eprintln!("服务器配置读取失败，将使用默认值：{}", e);
            AppConfig::default()
        });

        init_controller(appconfig)?;
        start_server()?;

        // //浏览器的配置及启动
        // let mut browser = BrowserServer::new(BrowserConfig::load("./config/browser.json")?)?;
        // browser.start().await?;
        // let browser_server_url = browser.get_server_url();

        // //解析小说页面的配置
        // let mut config = DynamicConfig::new();
        // config.load(PathBuf::from("./config/novel.json"))?;
        // config.with_set("browser-server", json!(browser_server_url));

        //加载并发数、检查轮数等配置
        self.load_config("./config/bilinovel.json")?;

        //下载小说
        for i in index {
            if i < self.volume.len() as u8 {
                if let Some(v) = self.volume.get_mut(i as usize) {
                    //浏览器的配置及启动
                    let mut browser =
                        BrowserServer::new(BrowserConfig::load("./config/browser.json")?)?;
                    browser.start().await?;
                    let browser_server_url = browser.get_server_url();

                    //解析小说页面的配置
                    let mut config = DynamicConfig::new();
                    config.load(PathBuf::from("./config/novel.json"))?;
                    config.with_set("browser-server", json!(browser_server_url));

                    let mut already = false;
                    // 检查是否有保存的状态
                    let state_path = format!("./temp/download/{}.state.json", v.name);
                    let images_path = format!("./temp/images/{}", v.name);
                    let novel_download_state = if Path::new(&state_path).exists() {
                        // 读取保存的状态
                        let content = std::fs::read_to_string(&state_path)?;
                        let saved_novel: Novel = serde_json::from_str(&content)?;

                        // 询问用户是否恢复下载
                        println!("发现未完成的下载:{}", v.name.to_owned().dark_yellow());
                        println!(
                            "- 已完成章节: {}",
                            saved_novel.chapters.len() - saved_novel.pending_chapter_indices.len()
                        );
                        println!(
                            "- 待完成章节: {}",
                            saved_novel.pending_chapter_indices.len()
                        );
                        println!("是否恢复下载? (y-恢复, n-重新开始)");

                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input)?;

                        if input.trim().to_lowercase() == "y" {
                            if saved_novel.pending_chapter_indices.len() == 0 {
                                already = true;
                            }
                            //恢复数据
                            *v = saved_novel.clone();
                            //读取失败跳过该卷
                            match load_images_from_file(&images_path) {
                                Ok(_) => {}
                                Err(e) => {
                                    eprintln!("读取图片数据失败：{}", e);
                                    // continue;
                                }
                            }
                            // 恢复下载：使用保存的状态
                            Arc::new(Mutex::new(saved_novel))
                        } else {
                            // 重新开始：创建新的状态
                            let new_state = v.clone();
                            Arc::new(Mutex::new(new_state))
                        }
                    } else {
                        // 创建新的状态
                        let new_state = v.clone();
                        Arc::new(Mutex::new(new_state))
                    };

                    println!("\n");
                    if !already {
                        println!("开始下载[按q中止下载]：{}", v.name.to_owned().dark_yellow());
                        match v
                            .parser_by_singlefile(
                                config.clone(),
                                self.config.max_concurrent,
                                &browser_server_url,
                                Arc::clone(&novel_download_state), // 传入 Arc<Mutex<Novel>>
                                &state_path,                       // 传入状态文件路径
                                &images_path,
                                self.config.compression_level,
                                self.config.save_interval,
                            )
                            .await
                        {
                            Ok(_) => {
                                info!("{}下载成功", v.name); // 下载完成，删除状态文件
                            }
                            Err(e) => {
                                error!("{}下载出错：{}", v.name, e);
                                eprintln!("{}下载出错：{}", v.name, e);
                                continue;
                            }
                        }
                    }
                    browser.stop();

                    let max_echo = self.config.check_rounds;

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
                    println!("是否开始检查章节?(y-继续)");
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;

                    if input.trim().to_lowercase() == "y" {
                        if Path::new("./config/http_check.json").exists() {
                            let appconfig_check = AppConfig::from_file("./config/http_check.json")
                                .unwrap_or_else(|e| {
                                    error!("从文件读取AppConfig配置失败，使用默认配置：{}", e);
                                    eprintln!("从文件读取AppConfig配置失败，使用默认配置：{}", e);
                                    AppConfig::default()
                                });
                            match update_config(|f| {
                                *f = appconfig_check;
                            }) {
                                Ok(_) => {
                                    info!("从{}读取配置成功", "./config/http_check.json");
                                }
                                Err(e) => {
                                    error!("从{}读取配置失败:{}", "./config/http_check.json", e);
                                }
                            }
                        }

                        //浏览器的配置及启动
                        let mut browser_check = BrowserServer::new(BrowserConfig::load(
                            "./config/browser_check.json",
                        )?)?;
                        browser_check.start().await?;
                        let browser_server_url_check = browser_check.get_server_url();

                        //解析小说页面的配置
                        let mut config_check = DynamicConfig::new();
                        config_check.load(PathBuf::from("./config/novel.json"))?;
                        config_check.with_set("browser-server", json!(browser_server_url));

                        for i in 1..=max_echo {
                            // 检查是否有按键命令
                            if cancelled.load(Ordering::Relaxed) {
                                println!("检查已被取消");
                                break;
                            }

                            println!(
                                "开始检查章节[没有问题将提前退出，按q退出检查]：第{}/{}轮检查……",
                                i, max_echo
                            );
                            match v
                                .check(
                                    config_check.clone(),
                                    &browser_server_url_check,
                                    self.config.check_concurrent,
                                    Arc::clone(&novel_download_state), // 传入 Arc<Mutex<Novel>>
                                    &state_path,                       // 传入状态文件路径
                                    &images_path,
                                    self.config.compression_level,
                                    self.config.save_interval,
                                )
                                .await
                            {
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
                        browser_check.stop();
                    }

                    // 停止按键监听器
                    let _ = stop_tx.send(()).await;
                    key_listener_handle.await?;

                    println!("是否开始打包epub?(y-继续)");
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;

                    if input.trim().to_lowercase() == "y" {
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
                        match generator.with_css(&self.config.css).generate_epub(file) {
                            Ok(_) => {
                                println!("生成成功！是否删除下载数据(y-删除)");
                                let mut input = String::new();
                                std::io::stdin().read_line(&mut input)?;

                                if input.trim().to_lowercase() == "y" {
                                    if Path::new(&state_path).exists() {
                                        std::fs::remove_file(&state_path).unwrap_or_else(|e| {
                                            eprintln!(
                                                "删除临时数据失败 file:{}  error:{}",
                                                state_path, e
                                            )
                                        });
                                    }
                                    if Path::new(&images_path).exists() {
                                        std::fs::remove_file(&images_path).unwrap_or_else(|e| {
                                            eprintln!(
                                                "删除临时数据失败 file:{}  error:{}",
                                                images_path, e
                                            )
                                        });
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("打包章节出错：{}", e);
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
