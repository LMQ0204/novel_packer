use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;
use anyhow::anyhow;

use super::config::{AppConfig,ImageData};
use super::server::HttpServer;
use crate::Result;

/// 主控制器，用于管理服务器和配置
pub struct Controller {
    config: Arc<RwLock<AppConfig>>,
    server_handle: RwLock<Option<thread::JoinHandle<()>>>,
    server: RwLock<Option<Arc<HttpServer>>>,
    should_stop: Arc<std::sync::atomic::AtomicBool>,
    // 使用HashMap存储图片数据，URL作为键
    images: Arc<RwLock<HashMap<String, ImageData>>>,
}

impl Controller {
    /// 创建新的控制器
    pub fn new(config: AppConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            server_handle: RwLock::new(None),
            server: RwLock::new(None),
            should_stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            images: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// 启动 HTTP 服务器
    pub fn start_server(&self) -> Result<()> {
        let mut server_handle = self.server_handle.write().unwrap();
        if server_handle.is_some() {
            return Err(anyhow!("Server is already running"));
        }
        
        let config = Arc::clone(&self.config);
        let should_stop = Arc::clone(&self.should_stop);
        let images = Arc::clone(&self.images);
        let server = HttpServer::new(Arc::clone(&config), Arc::clone(&should_stop), images)?;
        let server = Arc::new(server);
        
        // 保存服务器引用
        *self.server.write().unwrap() = Some(Arc::clone(&server));
        
        // 启动服务器线程
        let server_clone = Arc::clone(&server);
        let handle = thread::spawn(move || {
            if let Err(e) = server_clone.run() {
                eprintln!("Server error: {}", e);
            }
        });
        
        *server_handle = Some(handle);
        
        // 等待一段时间确保服务器启动
        thread::sleep(Duration::from_millis(100));
        
        Ok(())
    }
    
    /// 停止 HTTP 服务器
    pub fn stop_server(&self) -> Result<()> {
        // 设置停止标志
        self.should_stop.store(true, std::sync::atomic::Ordering::Relaxed);
        
        if let Some(server) = &*self.server.read().unwrap() {
            // 尝试停止服务器
            if let Err(e) = server.stop() {
                eprintln!("Failed to stop server gracefully: {}", e);
            }
            
            // 等待一段时间
            thread::sleep(Duration::from_millis(500));
        }
        
        // 等待线程结束
        if let Some(handle) = self.server_handle.write().unwrap().take() {
            // 设置超时，防止无限等待
            if handle.join().is_err() {
                eprintln!("Failed to join server thread");
            }
        }
        
        *self.server.write().unwrap() = None;
        Ok(())
    }
    
    /// 更新配置
    pub fn update_config<F>(&self, updater: F) -> Result<()>
    where
        F: FnOnce(&mut AppConfig),
    {
        let mut config = self.config.write().unwrap();
        updater(&mut config);
        Ok(())
    }
    
    /// 获取当前配置
    pub fn get_config(&self) -> AppConfig {
        self.config.read().unwrap().clone()
    }
    
    /// 检查服务器是否正在运行
    pub fn is_running(&self) -> bool {
        self.server_handle.read().unwrap().is_some()
    }
    
    /// 获取服务器端口
    pub fn get_port(&self) -> u16 {
        self.config.read().unwrap().server_port
    }
    
    /// 通过URL获取图片数据
    pub fn get_image_by_url(&self, url: &str) -> Option<ImageData> {
        self.images.read().unwrap().get(url).cloned()
    }
    
    /// 获取所有图片数据
    pub fn get_all_images(&self) -> HashMap<String, ImageData> {
        self.images.read().unwrap().clone()
    }
    
    /// 清空图片数据
    pub fn clear_images(&self) {
        self.images.write().unwrap().clear();
    }
    
    /// 删除特定URL的图片数据
    pub fn remove_image(&self, url: &str) -> Option<ImageData> {
        self.images.write().unwrap().remove(url)
    }
}