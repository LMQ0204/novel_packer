use crate::{core::singlefile::Singlefile, source::bilinovel::types::BiliNovel};
use once_cell::sync::Lazy;
use std::{collections::HashMap, sync::Mutex};

// 全局注册表
pub static URL_HANDLERS: Lazy<Mutex<HashMap<String, fn(String) -> Box<dyn Singlefile>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

// 注册 URL 处理器
pub fn register_url_handler(domain: &str, handler: fn(String) -> Box<dyn Singlefile>) {
    URL_HANDLERS
        .lock()
        .unwrap()
        .insert(domain.to_string(), handler);
}

// 初始化注册表（可以在程序启动时调用）
pub fn init_url_handlers() {
    register_url_handler("www.linovelib.com", |url| Box::new(BiliNovel::new(url)));
    register_url_handler("linovelib.com", |url| Box::new(BiliNovel::new(url)));
    // 注册更多处理器...
}

///初始化注册表
pub fn init_url_parser() {
    init_url_handlers();
}

/// 初始化日志系统
pub fn init_logger() {
    tracing_subscriber::fmt::init();
}