use reqwest;
// use std::error::Error;
use std::time::Duration;
use moka::future::Cache;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use tokio::time::sleep;
use anyhow::{Result, Context};

use crate::utils::httpclient::types::*;

pub struct AsyncHttpClient {
    client: reqwest::Client,
    user_agent: String,
    cache: Option<Cache<u64, HttpResponse>>,
    config: RequestConfig,
}

impl AsyncHttpClient {
    // 创建新的异步 HTTP 客户端实例
    pub fn new(config: RequestConfig) -> Result<Self> {
        // 创建异步客户端
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .connect_timeout(Duration::from_secs(10))
            .build()
            .context("创建 HTTP 客户端失败")?;
        
        // 创建异步缓存（默认缓存 1000 个响应，有效期 5 分钟）
        let cache = if config.max_retries == 0 {
            Some(
                Cache::builder()
                    .max_capacity(1000)
                    .time_to_live(Duration::from_secs(300))
                    .build()
            )
        } else {
            None
        };
        
        Ok(Self {
            client,
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36".to_string(),
            cache,
            config,
        })
    }
    
    // 设置自定义 User-Agent
    pub fn set_user_agent(&mut self, user_agent: String) {
        self.user_agent = user_agent;
    }
    
    // 生成请求的哈希键（用于缓存）
    fn generate_cache_key(&self, method: &str, url: &str, body: Option<&str>) -> u64 {
        let mut hasher = DefaultHasher::new();
        method.hash(&mut hasher);
        url.hash(&mut hasher);
        if let Some(body) = body {
            body.hash(&mut hasher);
        }
        hasher.finish()
    }
    
    // 检查缓存
    async fn check_cache(&self, key: u64) -> Option<HttpResponse> {
        match &self.cache {
            Some(cache) => cache.get(&key).await,
            None => None,
        }
    }
    
    // 存储到缓存
    async fn store_in_cache(&self, key: u64, response: HttpResponse) {
        if let Some(cache) = &self.cache {
            cache.insert(key, response).await;
        }
    }
    
    // 异步发送 GET 请求（带重试和缓存）
    pub async fn get(&self, url: &str) -> Result<HttpResponse> {
        let cache_key = self.generate_cache_key("GET", url, None);
        
        // 检查缓存
        if let Some(cached_response) = self.check_cache(cache_key).await {
            return Ok(cached_response);
        }
        
        let mut last_error = None;
        
        // 重试逻辑
        for attempt in 0..=self.config.max_retries {
            match self.execute_get(url).await {
                Ok(response) => {
                    // 存储到缓存
                    self.store_in_cache(cache_key, response.clone()).await;
                    return Ok(response);
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.config.max_retries {
                        println!("请求失败，第 {} 次重试...", attempt + 1);
                        sleep(self.config.retry_delay * (attempt + 1) as u32).await;
                    }
                }
            }
        }
        
        Err(last_error.unwrap())
    }
    
    // 执行 GET 请求（不包含重试逻辑）
    async fn execute_get(&self, url: &str) -> Result<HttpResponse> {
        let response = self.client
            .get(url)
            .header("User-Agent", &self.user_agent)
            .send()
            .await
            .context("发送 GET 请求失败")?;
        
        self.process_response(response).await
    }
    
    // 异步发送 POST 请求（带重试）
    pub async fn post(&self, url: &str, body: String) -> Result<HttpResponse> {
        let cache_key = self.generate_cache_key("POST", url, Some(&body));
        
        // 检查缓存（对于 POST 请求，通常不缓存，但这里提供了选项）
        if let Some(cached_response) = self.check_cache(cache_key).await {
            return Ok(cached_response);
        }
        
        let mut last_error = None;
        
        // 重试逻辑
        for attempt in 0..=self.config.max_retries {
            match self.execute_post(url, &body).await {
                Ok(response) => {
                    // 存储到缓存（对于 POST 请求，通常不缓存）
                    if self.config.max_retries == 0 { // 只有在禁用重试时才缓存 POST
                        self.store_in_cache(cache_key, response.clone()).await;
                    }
                    return Ok(response);
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.config.max_retries {
                        println!("请求失败，第 {} 次重试...", attempt + 1);
                        sleep(self.config.retry_delay * (attempt + 1) as u32).await;
                    }
                }
            }
        }
        
        Err(last_error.unwrap())
    }
    
    // 执行 POST 请求
    async fn execute_post(&self, url: &str, body: &str) -> Result<HttpResponse> {
        let response = self.client
            .post(url)
            .header("User-Agent", &self.user_agent)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body.to_string())
            .send()
            .await
            .context("发送 POST 请求失败")?;
        
        self.process_response(response).await
    }
    
    // 处理响应并提取信息
    async fn process_response(&self, response: reqwest::Response) -> Result<HttpResponse> {
        let status = response.status().as_u16();
        let url = response.url().to_string();
        
        // 提取响应头
        let headers = response.headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.to_string(),
                    value.to_str().unwrap_or("").to_string()
                )
            })
            .collect();
        
        // 获取内容类型
        let content_type = response.headers()
            .get("content-type")
            .and_then(|ct| ct.to_str().ok())
            .map(|s| s.to_string());
        
        // 获取响应体
        let body = response.text().await.context("读取响应体失败")?;
        
        Ok(HttpResponse {
            status,
            headers,
            body,
            url,
            content_type,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        })
    }
}

impl Default for AsyncHttpClient {
    fn default() -> Self {
        AsyncHttpClient::new(RequestConfig::default()).expect("创建默认异步http客户端失败!")
    }
}