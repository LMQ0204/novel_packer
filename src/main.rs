mod core;
mod source;
mod utils;
use std::io::stdin;

use anyhow::{Result};

use crate::utils::{browser::browser_server::{BrowserConfig, BrowserServer}, httpserver::{AppConfig, get_all_images, init_controller, start_server}};

#[tokio::main]
async fn main() -> Result<()> {
    let mut a = AppConfig::default();
    a.set_send_to_rust(true);
    a.set_regex_pattern(r"^https://img3\.readpai\.com/.*");
    init_controller(a)?;
    start_server()?;
    println!("回车停止...");
    stdin().read_line(&mut String::new())?;
    let b = get_all_images()?;
    for (url, image) in b {
        println!("url:{}",url);
        println!("filename:{}",image.filename);
        println!("type:{}",image.mime_type);
    }

    Ok(())
}