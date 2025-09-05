// #![cfg_attr(debug_assertions, allow(dead_code, unused_imports, unused_variables, unused_mut))]

mod core;
mod source;
mod utils;
use std::io::stdin;

use anyhow::Result;

use crate::{core::{get_struct::get_from_url, init::{init_logger, init_url_parser}}, utils::httpserver::get_all_images};

#[tokio::main]
async fn main() -> Result<()> {
    // a.set_send_to_rust(true);
    // a.set_regex_pattern(r"^https://img3\.readpai\.com/.*");
    // a.set_save_to_file(true);
    // a.set_open_download(true);
    // init_controller(a)?;
    // start_server()?;
    // println!("回车停止...");
    // stdin().read_line(&mut String::new())?;
    // let b = get_all_images()?;
    // for (url, image) in b {
    //     println!("url:{}",url);
    //     println!("filename:{}",image.filename);
    //     println!("type:{}",image.mime_type);
    // }
    // let mut browser = BrowserServer::new(BrowserConfig::load("old.json")?)?;
    // browser.start().await?;
    init_url_parser();
    #[cfg(debug_assertions)]
    init_logger();
    
    let mut a = get_from_url();
    a.check()?;
    a.display().await?;
    a.download().await?;
    // a.get_novel()?;
    
    // let browser_server_url = browser.get_server_url();
    // let mut appconfig = AppConfig::default();
    //     appconfig.set_send_to_rust(true);
    //     appconfig.set_regex_pattern(r"^https://img3\.readpai\.com/.*");
    //     appconfig.set_open_download(true);

    // init_controller(appconfig)?;
    // start_server()?;
    // let mut browser = BrowserServer::new(BrowserConfig::load("old.json")?)?;
    // browser.start().await?;
    // let images = get_all_images()?;
    // for (url, image) in images {
    //     println!("url:{}\tfilenme:{}",url, image.filename);
    // }
    // let url = "https://www.linovelib.com/novel/3095/274053_3.html";
    // let url = "https://www.linovelib.com/novel/2727/vol_129092.html";
    // let mut dconfig = DynamicConfig::new();
    // dconfig.with_set("remove-hidden-elements", json!(true))
    //         .with_set("output-directory",json!("E:\\Download\\AB\\Programs\\test_\\out"))
    //         // .with_set("browser-script",json!("show-content-.js"))
    //         .with_set("browser-script",json!("new.js"))
    //         .with_set("console-messages-file",json!("log-.json"))
    //         .with_set("save-raw-page",json!(false))
    //         .with_set("output-json",json!(false))
    //         .with_set("block-images",json!(false))
    //         .with_set("dump-content",json!(false))
    //         // .with_set("console-messages-file",json!("./log.json"))
    //         .with_set("browser-server",json!("http://localhost:9223"));
    // download_from_url(url, dconfig).await?;

    // let a = std::fs::read_to_string(r"E:\Download\AB\Programs\test_\out\败北女角太多了！ 第八卷 ～第一败～ 绝无任何企图（3）_哔哩轻小说 (2025_9_3 20：09：02).html")?;
    // std::fs::write("raw.txt", a.clone())?;
    // // let html = Html::parse_document(&a).html();
    // // std::fs::write("html.txt", html.clone())?;
    // let (image, text) = extract_chapter(&a, "div#TextContent","data-original-src",)?;
    // std::fs::write("text.txt", text.clone())?;
    // println!("{}",text);
    // println!("{:?}",image);

    // let v = download_chapter_singlefile("https://www.linovelib.com/novel/2727/292979.html","http://localhost:9223","value.json","new.js").await?;
    println!("回车停止...");
    let mut a = String::new();
    stdin().read_line(&mut a)?;
    
    let images = get_all_images()?;
    for (url, image) in images {
        println!("url:{}\tfilenme:{}",url, image.filename);
    }
    // let b= get_index_from_stdin(&a)?;
    // println!("{:?}",b);
    Ok(())
}
