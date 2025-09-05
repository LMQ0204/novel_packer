pub mod default_css;
use std::{
    collections::{HashMap, HashSet},
    io::Cursor,
};

use anyhow::Result;
use epub_builder::{EpubBuilder, EpubContent, ReferenceType, ZipLibrary};
use regex::Regex;
use std::io::{Seek, Write};

use crate::{
    source::bilinovel::types::{Chapter, Novel},
    utils::httpserver::ImageData,
};

pub struct EpubGenerator<'a> {
    novel: &'a Novel,
    images: &'a HashMap<String, ImageData>,
    css: Option<String>, 
}

impl<'a> EpubGenerator<'a> {
    pub fn new(novel: &'a Novel, images: &'a HashMap<String, ImageData>) -> Self {
        EpubGenerator { novel, images, css: None,}
        
    }

    // 添加设置自定义CSS的方法
    pub fn with_css(mut self, css: String) -> Self {
        self.css = Some(css);
        self
    }

    pub fn generate_epub<W: Write + Seek>(&self, output: W) -> Result<()> {
        let mut builder = EpubBuilder::new(ZipLibrary::new()?)?;
        builder.epub_version(epub_builder::EpubVersion::V30);

        // 设置元数据
        self.set_metadata(&mut builder)?;

        // 添加CSS样式表
        self.add_stylesheet(&mut builder)?;

        // 添加封面图片
        self.add_cover_image(&mut builder)?;

        // 添加所有章节中引用的图片资源
        self.add_chapter_images(&mut builder)?;

        // 添加封面页面
        self.add_cover_page(&mut builder)?;

        // 添加目录页面
        self.add_table_of_contents(&mut builder)?;

        // 添加章节内容
        self.add_chapters(&mut builder)?;

        builder.generate(output)?;
        Ok(())
    }

    fn set_metadata(&self, builder: &mut EpubBuilder<ZipLibrary>) -> Result<()> {
        builder.metadata("title", &self.novel.name)?;
        builder.metadata("author", &self.novel.author)?;
        builder.metadata("lang", "zh-CN")?;
        // builder.metadata("identifier", &Uuid::new_v4().to_string())?;
        // builder.metadata("date", &Utc::now().to_rfc3339())?;

        if !self.novel.description.is_empty() {
            builder.metadata("description", &self.novel.description)?;
        }

        if let Some(tags) = &self.novel.tags {
            if !tags.state.is_empty() {
                builder.metadata("subject", &tags.state)?;
            }
            for label in &tags.label {
                builder.metadata("subject", label)?;
            }
            for label in &tags.span {
                builder.metadata("subject", label)?;
            }
        }

        Ok(())
    }

    fn add_stylesheet(&self, builder: &mut EpubBuilder<ZipLibrary>) -> Result<()> {
        // 使用自定义CSS或默认CSS
        let css_content = match &self.css {
            Some(custom_css) => custom_css.as_str(),
            None => default_css::DEFAULT_CSS,
        };
        
        builder.add_resource("styles.css", Cursor::new(css_content), "text/css")?;
        Ok(())
    }

    fn add_cover_image(&self, builder: &mut EpubBuilder<ZipLibrary>) -> Result<()> {
        if let Some(image_data) = self.images.get(&self.novel.cover) {
            // 使用 Cursor 包装字节数据，使其实现 Read trait
            let reader = Cursor::new(&image_data.u8_data);
            builder.add_cover_image("cover.png", reader, &image_data.mime_type)?;
        }
        Ok(())
    }

    fn add_chapter_images(&self, builder: &mut EpubBuilder<ZipLibrary>) -> Result<()> {
        let mut added_images = HashSet::new(); // 用于跟踪已添加的图片

        for chapter in &self.novel.chapters {
            for image_url in &chapter.image {
                // 跳过封面图片，因为它已经单独添加了
                if image_url == &self.novel.cover {
                    continue;
                }

                if let Some(image_data) = self.images.get(image_url) {
                    // 检查是否已经添加过这个图片
                    if !added_images.contains(&image_data.filename) {
                        let path = format!("images/{}", image_data.filename);
                        // 使用 Cursor 包装字节数据，使其实现 Read trait
                        let reader = Cursor::new(&image_data.u8_data);
                        builder.add_resource(&path, reader, &image_data.mime_type)?;

                        // 记录已添加的图片
                        added_images.insert(image_data.filename.clone());
                    }
                }
            }
        }
        Ok(())
    }

    fn add_cover_page(&self, builder: &mut EpubBuilder<ZipLibrary>) -> Result<()> {
        let cover_content = r#"<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml">
<head>
    <title>封面</title>
    <link rel="stylesheet" type="text/css" href="styles.css" />
</head>
<body>
    <img lass="cover-image" src="cover.png" alt="封面图片" />
</body>
</html>"#;

        builder.add_content(
            EpubContent::new("cover.xhtml", cover_content.as_bytes())
                .title("封面")
                .reftype(ReferenceType::Cover),
        )?;
        Ok(())
    }

    fn add_table_of_contents(&self, builder: &mut EpubBuilder<ZipLibrary>) -> Result<()> {
        let mut toc_content = String::new();
        toc_content.push_str(
            r#"<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml">
<head>
    <title>目录</title>
    <link rel="stylesheet" type="text/css" href="styles.css" />
</head>
<body>
    <h1 class="toc-title">目录</h1>
    <ul class="toc-list">"#,
        );

        for (index, chapter) in self.novel.chapters.iter().enumerate() {
            let filename = format!("chapter_{:03}.xhtml", index + 1);
            toc_content.push_str(&format!(
                r#"<li class="toc-item"><a class="toc-link" href="{}">{}</a></li>"#,
                filename,
                escape_xml(&chapter.title)
            ));
        }

        toc_content.push_str(
            r#"</ul>
</body>
</html>"#,
        );

        builder.add_content(
            EpubContent::new("toc.xhtml", toc_content.as_bytes())
                .title("目录")
                .reftype(ReferenceType::Toc),
        )?;
        Ok(())
    }

    fn add_chapters(&self, builder: &mut EpubBuilder<ZipLibrary>) -> Result<()> {
        for (index, chapter) in self.novel.chapters.iter().enumerate() {
            let filename = format!("chapter_{:03}.xhtml", index + 1);
            let title = &chapter.title;

            // 构建完整的 XHTML 文档
            let content = self.build_chapter_content(chapter)?;

            let mut epub_content = EpubContent::new(&filename, content.as_bytes()).title(title);

            // 只有第一章标记为文本开始
            if index == 0 {
                epub_content = epub_content.reftype(ReferenceType::Text);
            }

            builder.add_content(epub_content)?;
        }
        Ok(())
    }

    fn build_chapter_content(&self, chapter: &Chapter) -> Result<String> {
        // 构建完整的 XHTML 文档
        Ok(format!(
            r#"<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml">
<head>
    <title>{}</title>
    <meta charset="UTF-8" />
    <link rel="stylesheet" type="text/css" href="styles.css" />
</head>
<body>
    <h1>{}</h1>
    {}
</body>
</html>"#,
            escape_xml(&chapter.title),
            escape_xml(&chapter.title),
            clean_html(&chapter.context.join(""))?
        ))
    }
}

// XML 转义函数
fn escape_xml(s: &str) -> String {
    s.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&apos;")
}

pub fn clean_html(html: &str) -> Result<String> {
    let mut cleaned = html.to_string();

    // 1. 移除HTML文档类型声明
    let doctype_re = Regex::new(r"(?i)<!DOCTYPE\s+html\b[^>]*>")?;
    cleaned = doctype_re.replace_all(&cleaned, "").to_string();

    // 2. 确保自闭合标签正确格式化
    let self_closing_re =
        Regex::new(r"(?i)<(\s*)(meta|link|img|br|hr|input)(\s+[^>]*?)?(\s*)/?(\s*)>")?;
    cleaned = self_closing_re
        .replace_all(&cleaned, |caps: &regex::Captures| {
            let tag_name = caps[2].to_lowercase();
            let attrs = caps.get(3).map_or("", |m| m.as_str());
            format!("<{} {} />", tag_name, attrs.trim())
        })
        .to_string();

    // 3. 补充XHTML命名空间
    let html_tag_re = Regex::new(r"(?i)<html\b([^>]*?)>")?;
    cleaned = html_tag_re
        .replace_all(&cleaned, |caps: &regex::Captures| {
            let existing_attrs = caps[1].to_string();
            if existing_attrs.contains("xmlns=") {
                format!("<html {}>", existing_attrs)
            } else {
                format!(
                    "<html xmlns=\"http://www.w3.org/1999/xhtml\" {}>",
                    existing_attrs
                )
            }
        })
        .to_string();

    // 4. 移除脚本标签
    let script_re = Regex::new(r"(?is)<script\b[^>]*>.*?</script>")?;
    cleaned = script_re.replace_all(&cleaned, "").to_string();

    Ok(cleaned)
}
