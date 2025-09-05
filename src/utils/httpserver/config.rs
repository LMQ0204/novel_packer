use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 图片数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageData {
    pub u8_data: Vec<u8>,
    pub base64_data: String,
    pub filename: String,
    pub mime_type: String,
    pub file_path: Option<String>, 
}

/// 应用程序配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub regex_pattern: String,
    pub output_path: PathBuf,
    pub wait_time: u32,
    pub send_to_rust: bool,
    pub server_port: u16,
    pub save_to_file: bool, 
    pub open_download: bool
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            regex_pattern: String::new(),
            output_path: PathBuf::from("downloads"),
            wait_time: 1000,
            send_to_rust: true,
            server_port: 8080,
            save_to_file: false, // 默认不保存到文件
            open_download: true
        }
    }
}

impl AppConfig {
    /// 设置正则表达式模式
    pub fn set_regex_pattern(&mut self, pattern: &str) {
        self.regex_pattern = pattern.to_string();
    }
    
    /// 设置输出路径
    pub fn set_output_path(&mut self, path: &str) {
        self.output_path = PathBuf::from(path);
    }
    
    /// 设置等待时间
    pub fn set_wait_time(&mut self, ms: u32) {
        self.wait_time = ms;
    }
    
    /// 设置是否发送到 Rust
    pub fn set_send_to_rust(&mut self, send: bool) {
        self.send_to_rust = send;
    }
    
    /// 设置服务器端口
    pub fn set_server_port(&mut self, port: u16) {
        self.server_port = port;
    }

    /// 设置是否保存图片到文件
    pub fn set_save_to_file(&mut self, save: bool) {
        self.save_to_file = save;
    }
    /// 设置是否开启扩展 
    pub fn set_open_download(&mut self, save: bool) {
        self.open_download = save;
    }
}