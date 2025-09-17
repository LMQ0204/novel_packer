

#[derive(Default)]
pub struct BiliNovel {
    pub url: String,
    pub catalog: String,
    pub book_name: String,
    pub author: String,
    pub tags: Option<Tags>,
    pub nums: String,
    pub notice: String,
    pub description: String,
    pub volume: Vec<Novel>,
    pub index: Vec<u8>,
    pub config: NovelConfig
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct  NovelConfig {
    pub max_concurrent: usize,
    pub check_rounds: usize,
    pub css: String,
    pub compression_level: u32,
    pub check_concurrent: usize,
    pub save_interval: usize
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Novel {
    pub url: String,
    pub name: String,
    pub author: String,
    pub cover: String,
    pub tags: Option<Tags>,
    pub description: String,
    pub chapters: Vec<Chapter>,
    pub pending_chapter_indices: Vec<usize>, 
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub url: String,
    pub title: String,
    pub context: Vec<String>, 
    pub image:Vec<String>
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Tags {
    pub state: String,
    pub label: Vec<String>,
    pub span: Vec<String>
}

impl Tags {
    pub fn new() -> Self {
        Tags::default()
    }
}


impl Chapter {
    pub fn new(url:&str, title: &str) -> Self {
        Self {
            url:url.to_string(),
            title: title.to_string(),
            ..Chapter::default()
        }
    }
}

impl Novel {
    pub fn new(url: String, name: String) -> Self {
        let mut res = Novel::default();
        res.url = url;
        res.name = name;
        res
    }
}

impl BiliNovel {
    pub fn new(url: String, catalog: String) -> Self {
        let mut res = BiliNovel::default();
        res.url = url;
        res.catalog = catalog;
        res
    }
}

use anyhow::{Result};
impl NovelConfig {
    /// 从文件读取配置
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        // 读取文件内容
        let content = std::fs::read_to_string(path)?;
        // 反序列化
        let config = serde_json::from_str(&content)?;
        Ok(config)
    }
}

use colored::Colorize;
use serde::{Deserialize, Serialize};
use core::fmt;
use std::path::Path;

impl fmt::Display for BiliNovel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}\n", format!("{}",self.book_name).bright_yellow().bold())?;
        writeln!(f, "{}\n",format!("{}",self.author).blue())?;
        if let Some(tags) = &self.tags {
            writeln!(f, "{}", tags)?;
        }
        if !self.nums.is_empty() {
            writeln!(f, "{}\n", self.nums)?;
        }
        if !self.notice.is_empty() {
            for line in self.notice.split("\n") {
                writeln!(f, "{}", format!("{}",line).on_truecolor(160, 125, 125))?
            }
            ;
        }
        if !self.description.is_empty() {
            writeln!(f, "\n{}\n", self.description)?;
        }
        for (i, v) in self.volume.iter().enumerate() {
            writeln!(f, "[{}]\t{}", i, format!("{}",v.name).underline())?;
        }
        Ok(())
    }
}

impl fmt::Display for Tags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}  ", format!("{}", self.state).on_truecolor(21, 164, 11))?;

        let (r, mut g1,mut g) = (151, 120, 20);
        for l in &self.label {
            write!(f, "{}  ", format!("{}", l).on_truecolor(r, g ,g))?;
            g = 151 - g1 as u8;
            g1 /= 2;
        }
        for s in &self.span {
            write!(f, "{}  ",s)?;
        }
        write!(f, "\n")?;
        Ok(())
    }
}