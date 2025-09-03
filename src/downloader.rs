use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};
use anyhow::{Context, Result, anyhow};
use indicatif::{ProgressBar, ProgressStyle};
use tokio::{fs, task, time};
use url::Url;
use crate::config::SingleFileConfig;

#[derive(Debug)]
pub struct NestedDownloader {
    base_config: SingleFileConfig,
    link_selector: Option<String>,
    max_depth: u32,
    visited: HashSet<String>,
    download_queue: HashMap<String, PathBuf>,
    progress: ProgressBar,
}

impl NestedDownloader {
    pub fn new(
        base_config: SingleFileConfig,
        max_depth: u32,
        progress: ProgressBar,
    ) -> Self {
        let link_selector = base_config.crawl_selector.clone();
        
        Self {
            base_config,
            link_selector,
            max_depth,
            visited: HashSet::new(),
            download_queue: HashMap::new(),
            progress,
        }
    }
    
    /// 启动嵌套下载
    pub async fn download(&mut self, start_url: &str) -> Result<HashMap<String, PathBuf>> {
        self.add_to_queue(start_url, 0)?;
        
        while !self.download_queue.is_empty() {
            let (url, output_path) = self.download_queue.drain().next().unwrap();
            
            // 下载当前页面
            self.progress.set_message(format!("下载: {}", url));
            self.download_page(&url, &output_path).await?;
            self.progress.inc(1);
            
            // 提取新链接
            if let Some(selector) = &self.link_selector {
                let current_depth = self.get_url_depth(&url);
                if current_depth < self.max_depth {
                    let links = self.extract_links(&output_path, selector).await?;
                    for link in links {
                        self.add_to_queue(&link, current_depth + 1)?;
                    }
                }
            }
        }
        
        Ok(self.visited.iter()
            .map(|url| (url.clone(), self.get_output_path(url)))
            .collect())
    }
    
    /// 添加 URL 到下载队列
    fn add_to_queue(&mut self, url: &str, depth: u32) -> Result<()> {
        let normalized = self.normalize_url(url)?;
        
        if !self.visited.contains(&normalized) && depth <= self.max_depth {
            self.visited.insert(normalized.clone());
            let output_path = self.get_output_path(&normalized);
            self.download_queue.insert(normalized, output_path);
            self.progress.set_length(self.progress.length().unwrap_or(0) + 1);
        }
        
        Ok(())
    }
    
    /// 下载单个页面
    async fn download_page(&self, url: &str, output_path: &Path) -> Result<()> {
        let exe_path = self.get_executable_path();
        let mut config = self.base_config.clone();
        config.url = Some(url.to_string());
        config.output = Some(output_path.to_path_buf());
        
        let args = config.to_args();
        
        let status = task::spawn_blocking(move || {
            Command::new(exe_path)
                .args(args)
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .status()
        })
        .await
        .context("线程错误")??;
        
        if !status.success() {
            anyhow::bail!("下载失败: {}", url);
        }
        
        Ok(())
    }
    
    
    /// 从 HTML 提取链接
    async fn extract_links(&self, file_path: &Path, selector: &str) -> Result<Vec<String>> {
        let content = fs::read_to_string(file_path)
            .await
            .context("读取HTML文件失败")?;
        
        let document = scraper::Html::parse_document(&content);
        
        // 手动处理选择器解析错误
        let selector = scraper::Selector::parse(selector)
            .map_err(|e| anyhow!(format!("CSS选择器解析失败: {:?}", e)))?;

        let base_url = if let Some(url) = &self.base_config.url {
            Url::parse(url).ok()
        } else {
            None
        };
        
        let links: Result<Vec<String>> = document
            .select(&selector)
            .filter_map(|el| el.value().attr("href"))
            .map(|href| self.resolve_url(href, base_url.as_ref()))
            .collect();
        
        links
    }
    
    /// 解析相对 URL
    fn resolve_url(&self, href: &str, base_url: Option<&Url>) -> Result<String> {
        if href.starts_with("http://") || href.starts_with("https://") {
            return Ok(href.to_string());
        }
        
        if let Some(base) = base_url {
            let resolved = base.join(href)
                .context("URL解析失败")?
                .to_string();
            Ok(resolved)
        } else {
            anyhow::bail!("无法解析相对URL: {}", href)
        }
    }
    
    /// 获取可执行文件路径
    fn get_executable_path(&self) -> PathBuf {
        #[cfg(target_os = "windows")]
        return PathBuf::from("single-file.exe");
        
        #[cfg(not(target_os = "windows"))]
        return PathBuf::from("single-file");
    }
    
    /// 生成输出路径
    fn get_output_path(&self, url: &str) -> PathBuf {
        let mut path = PathBuf::from("downloads");
        
        // 创建深度目录
        let depth = self.get_url_depth(url);
        path.push(format!("depth_{}", depth));
        
        // 从 URL 生成文件名
        let mut filename = url
            .replace("https://", "")
            .replace("http://", "")
            .replace('/', "_")
            .replace('?', "_")
            .replace('=', "_")
            .replace(':', "_");
        
        // 截断过长的文件名
        if filename.len() > 150 {
            filename = format!("{}...{}", 
                &filename[..50], 
                &filename[filename.len()-50..]
            );
        }
        
        path.push(format!("{}.html", filename));
        path
    }
    
    /// 获取 URL 深度
    fn get_url_depth(&self, url: &str) -> u32 {
        url.matches('/').count() as u32
    }
    
    /// 标准化 URL
    fn normalize_url(&self, url: &str) -> Result<String> {
        let mut parsed = Url::parse(url)?;
        
        // 移除 fragment 和 query
        parsed.set_fragment(None);
        parsed.set_query(None);
        
        // 先获取路径字符串
        let path_str = parsed.path().to_string();
        let trimmed_path = path_str.trim_end_matches('/');
        
        // 然后设置路径
        parsed.set_path(trimmed_path);
        
        Ok(parsed.to_string())
    }
}