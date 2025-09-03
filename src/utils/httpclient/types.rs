use serde::{Deserialize, Serialize};
use std::time::Duration;


// 定义响应结构体
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub url: String,
    pub content_type: Option<String>,
    pub timestamp: u64, // 响应时间戳
}

impl HttpResponse {
    pub fn is_success(&self) -> bool {
        200 <= self.status &&self.status < 300
    }
}

// 请求配置
#[derive(Debug, Clone)]
pub struct RequestConfig {
    pub max_retries: u32,
    pub retry_delay: Duration,
    pub timeout: Duration,
}

impl Default for RequestConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay: Duration::from_secs(1),
            timeout: Duration::from_secs(30),
        }
    }
}