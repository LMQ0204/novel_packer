#![cfg_attr(debug_assertions, allow(dead_code))]

mod core;
mod source;
mod utils;
use crate::core::{get_struct::get_from_url, init::init_url_parser};
use anyhow::Result;
use chrono::Local;
use std::{env, io::stdin};
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[tokio::main]
async fn main() -> Result<()> {
    init_url_parser();

    // init_logger();
    // 检查是否应该输出到文件
    let log_to_file = env::var("LOG_TO_FILE")
        .map(|val| val == "1" || val.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    // 新增：检查是否应该输出到控制台（默认为true）
    let log_to_console = env::var("LOG_TO_CONSOLE")
        .map(|val| val != "1" && !val.eq_ignore_ascii_case("true")) // 默认true，除非明确设置为false或0
        .unwrap_or(false);

    let filter_layer = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let _guard = if log_to_file {
        // 文件日志配置
        let file_appender = tracing_appender::rolling::daily("./temp/logs", "app.log");
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        let mut layers = Vec::new();

        // 只有当需要输出到控制台时才添加控制台层
        if log_to_console {
            let console_layer = fmt::layer().with_target(true).with_level(true);
            layers.push(console_layer.boxed());
        }

        // 添加文件层
        let file_layer = fmt::layer()
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_target(true)
            .with_level(true);
        layers.push(file_layer.boxed());

        // 初始化订阅者
        tracing_subscriber::registry()
            .with(filter_layer)
            .with(layers)
            .init();

        Some(guard)
    } else {
        // 只有当需要输出到控制台时才初始化控制台输出
        if log_to_console {
            let fmt_layer = fmt::layer().with_target(true).with_level(true);

            tracing_subscriber::registry()
                .with(filter_layer)
                .with(fmt_layer)
                .init();
        } else {
            // 如果既不要控制台也不要文件，至少初始化一个空的订阅者
            tracing_subscriber::registry().with(filter_layer).init();
        }

        None
    };
    write_startup_info();

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

fn write_startup_info() {
    let now = Local::now();
    let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();

    // 获取程序名称和版本
    let program_name = env::args().next().unwrap_or_else(|| "unknown".to_string());
    // let program_version = env!("CARGO_PKG_VERSION", "CARGO_PKG_VERSION not set");

    // 写入分隔符和启动信息
    tracing::info!("================================================");
    tracing::info!("程序启动: {}", program_name);
    // tracing::info!("版本: {}", program_version);
    tracing::info!("时间: {}", timestamp);
    tracing::info!("进程ID: {}", std::process::id());
    tracing::info!(
        "工作目录: {:?}",
        std::env::current_dir().unwrap_or_default()
    );
    tracing::info!("运行参数: {:?}", std::env::args().collect::<Vec<String>>());
    tracing::info!("================================================");
}
