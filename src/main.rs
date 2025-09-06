#![cfg_attr(debug_assertions, allow(dead_code))]

mod core;
mod source;
mod utils;
use std::io::stdin;
use anyhow::Result;

use crate::{core::{get_struct::get_from_url, init::{init_logger, init_url_parser}}, utils::httpserver::get_all_images};

#[tokio::main]
async fn main() -> Result<()> {

    init_url_parser();
    #[cfg(debug_assertions)]
    init_logger();
    
    let mut novel = get_from_url();

    if let Err(e) = novel.check() {
        eprintln!("{}", e);
        return wait_for_exit();
    }
    

    if let Err(e) = novel.display().await {
        eprintln!("{}", e);
        return wait_for_exit();
    }
    
    if let Err(e) = novel.download().await {
        eprintln!("{}", e);
        return wait_for_exit();
    }
    

    let images = match get_all_images() {
        Ok(imgs) => imgs,
        Err(e) => {
            eprintln!("{}", e);
            return wait_for_exit();
        }
    };
    for (url, image) in images {
        println!("url:{}\tfilename:{}", url, image.filename);
    }
    
    // 所有步骤成功，正常等待退出
    wait_for_exit()
}

// 提取等待退出的逻辑为单独函数，复用
fn wait_for_exit() -> Result<()> {
    println!("按回车退出...");
    let mut input = String::new();
    stdin().read_line(&mut input)?; // 等待用户输入
    Ok(())
}