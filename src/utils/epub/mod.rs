use anyhow::{anyhow, Result};
use epub_builder::{EpubBuilder, EpubContent, ReferenceType, ZipLibrary};
use regex::Regex;
use std::io::{self, Cursor};
use std::path::Path;

/// EPUB 配置选项
#[derive(Clone, Debug)]
pub struct EpubConfig {
    pub title: String,
    pub author: String,
    pub language: String,
    pub css: Option<String>,
    pub add_cover: bool,
    pub add_toc: bool,
}

impl Default for EpubConfig {
    fn default() -> Self {
        Self {
            title: "Untitled".to_string(),
            author: "Unknown".to_string(),
            language: "zh-CN".to_string(),
            css: None,
            add_cover: true,
            add_toc: true,
        }
    }
}

/// EPUB 书籍构建器
pub struct EpubBook {
    config: EpubConfig,
    chapters: Vec<Chapter>,
}

/// 章节结构
#[derive(Clone, Debug)]
pub struct Chapter {
    pub title: String,
    pub content: String,
    pub filename: Option<String>,
}

impl Chapter {
    /// 创建新章节
    pub fn new<T: Into<String>, C: Into<String>>(title: T, content: C) -> Self {
        Self {
            title: title.into(),
            content: content.into(),
            filename: None,
        }
    }
    
    /// 设置自定义文件名
    pub fn with_filename<T: Into<String>>(mut self, filename: T) -> Self {
        self.filename = Some(filename.into());
        self
    }
}

impl EpubBook {
    /// 创建新的 EPUB 书籍
    pub fn new<T: Into<String>, A: Into<String>>(title: T, author: A) -> Self {
        Self {
            config: EpubConfig {
                title: title.into(),
                author: author.into(),
                ..Default::default()
            },
            chapters: Vec::new(),
        }
    }
    
    /// 使用配置创建 EPUB 书籍
    pub fn with_config(config: EpubConfig) -> Self {
        Self {
            config,
            chapters: Vec::new(),
        }
    }
    
    /// 添加章节
    pub fn add_chapter(&mut self, chapter: Chapter) -> &mut Self {
        self.chapters.push(chapter);
        self
    }
    
    /// 添加多个章节
    pub fn add_chapters(&mut self, chapters: Vec<Chapter>) -> &mut Self {
        self.chapters.extend(chapters);
        self
    }
    
    /// 设置配置
    pub fn set_config(&mut self, config: EpubConfig) -> &mut Self {
        self.config = config;
        self
    }
    
    /// 生成 EPUB 文件
    pub fn generate<P: AsRef<Path>>(&self, output_path: P) -> Result<()> {
        // 初始化 EPUB 构建器
        let zip = ZipLibrary::new()?;
        let mut builder = EpubBuilder::new(zip)?;
        
        // 设置元数据
        builder.metadata("title", &self.config.title)?;
        builder.metadata("author", &self.config.author)?;
        
        // 添加样式
        let css_content = self.config.css.as_deref().unwrap_or(DEFAULT_CSS);
        builder.stylesheet(Cursor::new(css_content.as_bytes()))?;
        
        // 添加封面
        if self.config.add_cover {
            self.add_cover_page(&mut builder)?;
        }
        
        // 添加目录
        if self.config.add_toc && !self.chapters.is_empty() {
            self.add_table_of_contents(&mut builder)?;
        }
        
        // 添加章节
        for (i, chapter) in self.chapters.iter().enumerate() {
            self.add_chapter_to_builder(&mut builder, chapter, i)?;
        }
        
        // 生成 EPUB 文件
        let mut file = std::fs::File::create(output_path)?;
        builder.generate(&mut file)?;
        
        Ok(())
    }
    
    /// 添加封面页
    fn add_cover_page(&self, builder: &mut EpubBuilder<ZipLibrary>) -> Result<()> {
        let cover_content = format!(
            r#"<div style="text-align: center; margin-top: 30%;">
                <h1>{}</h1>
                <h2>{}</h2>
            </div>"#,
            self.config.title, self.config.author
        );
        
        let cover_xhtml = wrap_xhtml("封面", &cover_content);
        
        // 使用链式调用，因为 title 和 reftype 方法会消耗 self 并返回新的实例
        let cover = EpubContent::new("cover.xhtml", Cursor::new(cover_xhtml.as_bytes()))
            .title("封面")
            .reftype(ReferenceType::Cover);
        
        builder.add_content(cover)?;
        
        Ok(())
    }
    
    /// 添加目录
    fn add_table_of_contents(&self, builder: &mut EpubBuilder<ZipLibrary>) -> Result<()> {
        let mut toc_items = String::new();
        for (i, chapter) in self.chapters.iter().enumerate() {
            let default_filename = format!("chapter_{}.xhtml", i + 1);
            let filename = chapter.filename.as_deref()
                .unwrap_or(&default_filename);
                
            toc_items.push_str(&format!(
                "<li><a href=\"{}\">{}</a></li>",
                filename, chapter.title
            ));
        }
        
        let toc_html = format!(
            r#"<ol>{}</ol>"#,
            toc_items
        );
        
        let toc_xhtml = wrap_xhtml("目录", &toc_html);
        
        // 使用链式调用
        let toc = EpubContent::new("toc.xhtml", Cursor::new(toc_xhtml.as_bytes()))
            .title("目录");
        
        builder.add_content(toc)?;
        
        Ok(())
    }
    
    /// 添加章节到构建器
    fn add_chapter_to_builder(
        &self, 
        builder: &mut EpubBuilder<ZipLibrary>, 
        chapter: &Chapter, 
        index: usize
    ) -> Result<()> {
        let default_filename = format!("chapter_{}.xhtml", index + 1);
        let filename = chapter.filename.as_deref()
            .unwrap_or(&default_filename);
            
        let xhtml_content = wrap_xhtml(&chapter.title, &chapter.content);
        
        // 使用链式调用
        let content = EpubContent::new(filename, Cursor::new(xhtml_content.as_bytes()))
            .title(&chapter.title)
            .reftype(ReferenceType::Text);
        
        builder.add_content(content)?;
        
        Ok(())
    }
}

// 默认 CSS 样式
const DEFAULT_CSS: &str = r#"
    body { 
        font-family: serif; 
        margin: 5%;
        line-height: 1.6;
    }
    h1, h2, h3 { 
        text-align: center; 
        page-break-after: avoid;
    }
    p {
        text-align: justify;
        text-indent: 2em;
        margin-bottom: 0.5em;
    }
    img {
        max-width: 100%;
        height: auto;
    }
"#;

/// 清理 HTML 内容，使其符合 EPUB XHTML 标准
pub fn clean_html(html: &str) -> String {
    // 移除 HTML 文档类型声明
    let re = Regex::new(r"(?i)<!DOCTYPE html\s*\[[^\]]*\]?>").unwrap();
    let cleaned = re.replace_all(html, "");
    
    // 确保自闭合标签正确格式化
    let re = Regex::new(r"(?i)<(meta|link|img|br|hr|input)([^>]*)>").unwrap();
    let cleaned = re.replace_all(&cleaned, |caps: &regex::Captures| {
        format!("<{} {} />", &caps[1], &caps[2])
    });
    
    cleaned.to_string()
}

/// 清理并包装 HTML 为完整的 XHTML 文档
fn wrap_xhtml(title: &str, content: &str) -> String {
    let cleaned_content = clean_html(content);
    
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.1//EN" "http://www.w3.org/TR/xhtml11/DTD/xhtml11.dtd">
<html xmlns="http://www.w3.org/1999/xhtml" xml:lang="zh-CN">
<head>
    <title>{}</title>
    <link rel="stylesheet" type="text/css" href="style.css" />
</head>
<body>
    <h1>{}</h1>
    {}
</body>
</html>"#,
        title, title, cleaned_content
    )
}

// 为 EpubBook 提供便捷的构建方法
impl EpubBook {
    /// 快速创建并生成 EPUB
    pub fn create_epub<P, T, A>(
        title: T,
        author: A,
        chapters: Vec<Chapter>,
        output_path: P,
    ) -> Result<()>
    where
        P: AsRef<Path>,
        T: Into<String>,
        A: Into<String>,
    {
        let mut book = Self::new(title, author);
        book.add_chapters(chapters);
        book.generate(output_path)
    }
}