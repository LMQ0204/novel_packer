
use crate::core::get_index::get_index_from_stdin;
use crate::core::singlefile::Singlefile;
use crate::source::bilinovel::types::BiliNovel;
use crate::utils::browser::browser_server::{BrowserConfig, BrowserServer};
use crate::utils::config::DynamicConfig;
use crate::utils::epub::EpubGenerator;
use crate::utils::httpclient::types::RequestConfig;
use crate::utils::httpserver::{AppConfig, get_all_images, init_controller, start_server};
use crate::utils::terminal::clear_previous_line;
use anyhow::Result;
use serde_json::json;

use async_trait::async_trait; // 导入宏

#[async_trait]
impl Singlefile for BiliNovel {
    async fn display(&mut self) -> Result<()> {
        // let user_input = read_url_from_stdin();
        // let mut book = BiliNovel::new(user_input);
        // let config = DynamicConfig::default();
        // book.parser_book_singlefile(config).await?;
        let config = RequestConfig::default();
        // let mut dconfig = DynamicConfig::new();
        // dconfig.with_set("remove-hidden-elements", json!(false))
        //        .with_set("output-directory",json!("E:\\Download\\AB\\Programs\\test_\\out"))
        //        .with_set("browser-script",json!("show-content.js"))
        //        .with_set("output-json",json!(true))
        //        .with_set("dump-content",json!(true))
        //        .with_set("browser-server",json!("http://localhost:9223"));
        self.parser_book_http_async(config).await?;
        // self.parser_book_singlefile(dconfig).await?;
        println!("{}", &self);
        println!("--------------------");
        Ok(())
    }

    fn init(&mut self) -> Result<()> {
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
        let mut appconfig = AppConfig::default();
        appconfig.set_send_to_rust(true);
        appconfig.set_regex_pattern(r"^https://img3\.readpai\.com/.*");
        appconfig.set_open_download(true);

        init_controller(appconfig)?;
        start_server()?;
        let mut browser = BrowserServer::new(BrowserConfig::load("old.json")?)?;
        browser.start().await?;
        let browser_server_url = browser.get_server_url();
        // let browser_server_url = format!("http://localhost:{}",port);
        let mut config = DynamicConfig::new();
        config
            .with_set("remove-hidden-elements", json!(false))
            .with_set("browser-server", json!(browser_server_url))
            .with_set("dump-content", json!(true))
            .with_set("output-json", json!(true))
            .with_set("block-images", json!(true))
            .with_set("browser-script", json!("new.js"));

        for i in index {
            if i < self.volume.len() as u8 {
                if let Some(v) = self.volume.get_mut(i as usize) {
                    println!("开始下载 [{}]...", v.name);
                    v.parser_by_singlefile(
                        config.clone(),
                        7,
                        &browser_server_url,
                        "sync_session_file/novel",
                        "new.js",
                    )
                    .await?;

                    // 创建取消标志
                    // let cancelled = Arc::new(AtomicBool::new(false));
                    // let cancelled_clone = cancelled.clone();

                    // println!("开始检查章节，输入 'cancel' 可取消任务（其他输入将被忽略）");
                    let max_echo = 100;
                    for i in 1..=max_echo {
                        // 检查是否收到取消请求
                        // if cancelled.load(Ordering::SeqCst) {
                        //     println!("章节检查被用户取消！");
                        //     break;
                        // }
                        println!("第{}/{}轮检查……",i,max_echo);
                        match v
                            .check_images(&browser_server_url, "images.json", "image.js")
                            .await
                        {
                            Ok(_) => {
                                println!("检查章节完成！");
                                break;
                            }
                            Err(e) => eprintln!("检查出错 {}", e),
                        }
                    }

                    // // 在单独线程中处理用户输入
                    // thread::spawn(move || {
                    //     let stdin = io::stdin();
                    //     for line in stdin.lock().lines() {
                    //         let input = match line {
                    //             Ok(s) => s.trim().to_lowercase(),
                    //             Err(_) => continue,
                    //         };

                    //         match input.as_str() {
                    //             "cancel" => {
                    //                 println!("正在取消任务...");
                    //                 cancelled_clone.store(true, Ordering::SeqCst);
                    //                 break;
                    //             }
                    //             _ => {
                    //                 println!("未知命令。可用命令: 'cancel'");
                    //             }
                    //         }
                    //     }
                    // });
                }
            }
        }
        Ok(())
    }

    fn get_novel(&self) -> Result<()> {
        let images = get_all_images()?;
        for i in self.index.to_owned() {
            if i < self.volume.len() as u8 {
                println!("打包中……");
                let generator = EpubGenerator::new(&self.volume[i as usize], &images);
                let file =
                    std::fs::File::create(format!("{}.epub", &self.volume[i as usize].name))?;
                generator.generate_epub(file)?;
            }
        }

        Ok(())
    }
}
