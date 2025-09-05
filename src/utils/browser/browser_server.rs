use reqwest::Client;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::time;

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

/// 浏览器服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    /// 浏览器可执行文件路径
    pub executable_path: String,
    /// 远程调试端口
    pub port: u16,
    /// 用户数据目录
    pub user_data_dir: String,
    /// 是否使用无头模式
    pub headless: bool,
    /// 额外的浏览器命令行参数
    #[serde(default)]
    pub additional_args: Vec<String>,
    /// 启动超时时间（秒）
    #[serde(default = "default_timeout")]
    pub startup_timeout: u64,
    /// 健康检查间隔（秒）
    #[serde(default = "default_health_check_interval")]
    pub health_check_interval: u64,
}

fn default_timeout() -> u64 {
    30
}

fn default_health_check_interval() -> u64 {
    5
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            executable_path: r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
                .to_string(),
            port: 9223,
            user_data_dir: r"E:\BrowserProfiles".to_string(),
            headless: true,
            additional_args: vec![
                "--no-sandbox".to_string(),
                "--disable-dev-shm-usage".to_string(),
            ],
            startup_timeout: 30,
            health_check_interval: 5,
        }
    }
}

impl BrowserConfig {
    /// 从文件加载配置
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let mut file = File::open(path).with_context(|| format!("无法打开配置文件: {:?}", path))?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .with_context(|| format!("读取配置文件失败: {:?}", path))?;

        let config: Self = serde_json::from_str(&contents)
            .with_context(|| format!("解析JSON配置失败: {:?}", path))?;

        Ok(config)
    }

    /// 保存配置到文件
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        let json_str = serde_json::to_string_pretty(self).context("序列化配置失败")?;

        let mut file =
            File::create(path).with_context(|| format!("创建配置文件失败: {:?}", path))?;

        file.write_all(json_str.as_bytes())
            .with_context(|| format!("写入配置文件失败: {:?}", path))?;

        Ok(())
    }

    /// 转换为命令行参数
    pub fn to_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        // 添加固定参数
        args.push(format!("--remote-debugging-port={}", self.port));
        args.push(format!("--user-data-dir={}", self.user_data_dir));

        if self.headless {
            args.push("--headless".to_string());
        }

        // 添加额外参数
        args.extend(self.additional_args.clone());

        args
    }
}

/// 浏览器标签信息
// #[derive(Debug, Deserialize)]
// pub struct BrowserTab {
//     pub id: String,
//     pub title: String,
//     pub url: String,
//     #[serde(rename = "type")]
//     pub tab_type: String,
// }

/// 浏览器服务器管理
pub struct BrowserServer {
    process: Option<Child>,
    config: BrowserConfig,
}

impl BrowserServer {
    /// 创建新的浏览器服务器实例
    pub fn new(config: BrowserConfig) -> Result<Self> {
        // 确保用户数据目录存在
        std::fs::create_dir_all(&config.user_data_dir).context("无法创建用户数据目录")?;

        Ok(BrowserServer {
            process: None,
            config,
        })
    }

    /// 启动浏览器服务器
    pub async fn start(&mut self) -> Result<()> {
        // 先停止可能已存在的实例
        self.stop();

        // println!("正在启动浏览器服务器...");

        let mut command = Command::new(&self.config.executable_path);
        command.args(self.config.to_args());
 
        command.stdout(Stdio::null()).stderr(Stdio::null());

        let child = command.spawn().context("启动浏览器进程失败")?;

        self.process = Some(child);

        // 等待服务器启动
        self.wait_until_ready()
            .await
            .context("浏览器服务器启动失败")?;

        // println!("浏览器服务器已启动在端口 {}", self.config.port);
        Ok(())
    }

    /// 等待浏览器服务器准备就绪
    async fn wait_until_ready(&self) -> Result<()> {
        let client = Client::new();
        let url = format!("http://localhost:{}/json/version", self.config.port);
        let start_time = std::time::Instant::now();
        let timeout = Duration::from_secs(30);

        while start_time.elapsed() < timeout {
            match client.get(&url).send().await {
                Ok(response) if response.status().is_success() => return Ok(()),
                _ => time::sleep(Duration::from_millis(500)).await,
            }
        }

        Err(anyhow!("浏览器服务器启动超时"))
    }

    /// 检查浏览器服务器是否正在运行
    pub async fn is_running(&self) -> bool {
        let client = Client::new();
        let url = format!("http://localhost:{}/json/version", self.config.port);

        match client.get(&url).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    /// 停止浏览器服务器
    pub fn stop(&mut self) {
        if let Some(mut process) = self.process.take() {
            // 先尝试正常终止
            let _ = process.kill();

            // 等待进程结束
            match process.wait() {
                Ok(_status) => {
                    // println!("浏览器进程已终止，退出状态: {}", status)
                }
                Err(e) => eprintln!("等待浏览器进程终止时出错: {}", e),
            }
        }
    }

    /// 获取浏览器服务器地址
    pub fn get_server_url(&self) -> String {
        format!("http://localhost:{}", self.config.port)
    }

    /// 获取配置信息
    pub fn get_config(&self) -> &BrowserConfig {
        &self.config
    }
}

impl Drop for BrowserServer {
    fn drop(&mut self) {
        self.stop();
    }
}
