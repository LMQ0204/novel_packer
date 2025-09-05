use std::path::Path;

use anyhow::{Result, anyhow};
use scraper::{Html, Selector};
use tracing::error;
use url::Url;
use visdom::Vis;

use crate::{source::bilinovel::types::{Chapter, Novel, Tags}, utils::epub::BROKEN_IMAGE_BASE64};

///提取章节内容，并处理img标签
pub fn extract_chapter(html: &str, selector: &str, src_name: &str, remove_vec: Vec<&str>) -> Result<(Vec<String>, String)> {
    let html = Vis::load(Html::parse_document(html).html()).map_err(|e| anyhow!("html解析失败 {}", e))?;
    let text = html.find(selector).first();
    let items = text.children("");
    // items.filter("div").remove();
    items.filter("p.sf-hidden").remove();
    
    for se in remove_vec {
        items.filter(se).remove();
    }
    // items.filter_by(handle)
    let mut src_vec = Vec::new();
    text.find("img").for_each(|_index, ele| {
        let mut img_ele = Vis::dom(ele);
        // 2. 检查是否存在目标属性（src_name，比如 "data-src" 或 "src"）
        if img_ele.has_attr(src_name) {
            if let Some(attr_value) = img_ele.attr(src_name) {
                let url = match Url::parse(&attr_value.to_string()) {
                    Ok(u) => u.to_string(),
                    Err(_) => String::new(),
                };
                let filename = Path::new(&url)
                    .file_name() // 取最后一个组件（如"a/b/c.jpg"→"c.jpg"）
                    .and_then(|os_str| os_str.to_str()) // 转&str（处理非UTF8）
                    .unwrap_or(""); // 任何错误都返回空字符串

                let src_val = attr_value.to_string();
                if !filename.is_empty() {
                    img_ele.set_attr("src", Some(&format!("images/{}", filename)));
                    src_vec.push(src_val);
                } else {
                    img_ele.set_attr("src", Some(BROKEN_IMAGE_BASE64));
                    src_vec.push(src_val);
                }

                // img_ele.set_attr("src", Some(&src_val));
                // src_vec.push(src_val);
            }
        } else {
            img_ele.set_attr("src", Some(BROKEN_IMAGE_BASE64));
        }
        true
    });
    Ok((src_vec, text.html().trim().to_string()))
}

///提取作者名称
pub fn extract_author(html: &str, selector: &str) -> Result<String> {
    let html = Html::parse_document(html);
    let se = Selector::parse(selector).map_err(|e| {
            error!("{}", e);
            anyhow!("{e}")
        })?;
    let author = html.select(&se).next().map_or_else(||String::new(), |s| s.text().collect::<String>());
    Ok(author)
}

///填充chapter的url、title
pub fn build_chapter(html: &str, selector: &str) -> Result<Vec<Chapter>> {
    let html = Vis::load(Html::parse_document(html).html()).map_err(|e| anyhow!("html解析失败 {}", e))?;
    let chapter = html.find(selector).first();
    let mut items = chapter.children("");
    let mut res = Vec::new();
    items.for_each(|_index, ele| {
        let chapter_ele = Vis::dom(ele);
        let url = chapter_ele.children("").first().attr("href").map_or_else(||String::new(), |v| v.to_string());
        let title = chapter_ele.children("").first().text();
        res.push(Chapter::new(&url, &title));
        true
    });
    Ok(res)
}

///提取描述
pub fn extract_description(html: &str, selector: &str) -> Result<(String, String, String)> {
    let html = Html::parse_document(html);
    let selector = Selector::parse(selector).map_err(|e| {
        error!("{}", e);
        anyhow!("{e}")
    })?;
    let nums_selector = Selector::parse("div.nums > span").map_err(|e| {
        error!("{}", e);
        anyhow!("{e}")
    })?;

    let des_selector = Selector::parse("div.book-dec>p").map_err(|e| {
        error!("{}", e);
        anyhow!("{e}")
    })?;

    let notice_selector = Selector::parse("div.notice").map_err(|e| {
        error!("{}", e);
        anyhow!("{e}")
    })?;

    let mut nums = String::new();
    let mut notice = String::new();
    let mut description = String::new();
    if let Some(element) = html.select(&selector).next() {
        let nums_txt = element
            .select(&nums_selector)
            .map(|span| {
                span.text()
                    .fold(String::new(), |mut s, t| {
                        s.push_str(t);
                        s
                    })
                    .trim()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\t");
        let des = element
            .select(&des_selector)
            .next()
            .map_or_else(
                || String::new(),
                |s| {
                    html2text::from_read(s.html().as_bytes(), 100).unwrap_or("提取失败".to_string())
                },
            )
            .trim()
            .to_string();
        let notice_txt = element
            .select(&notice_selector)
            .next()
            .map_or_else(
                || String::new(),
                |s| {
                    html2text::from_read(s.html().as_bytes(), 100).unwrap_or("提取失败".to_string())
                },
            )
            .trim()
            .to_string();

        // println!("nums:{nums}");
        nums.push_str(&nums_txt);
        description.push_str(&des);
        notice.push_str(&notice_txt);
    }

    Ok((nums, notice, description))
}

///提取标签
pub fn extract_tags(html: &str, selector: &str) -> Result<Tags> {
    let html = Html::parse_document(html);
    let selector = Selector::parse(selector).map_err(|e| {
        error!("{}", e);
        anyhow!("{e}")
    })?;
    let state_selector = Selector::parse("a.state").map_err(|e| {
        error!("{}", e);
        anyhow!("{e}")
    })?;
    let label_selector = Selector::parse("a.label").map_err(|e| {
        error!("{}", e);
        anyhow!("{e}")
    })?;
    let span_selector = Selector::parse("span>a").map_err(|e| {
        error!("{}", e);
        anyhow!("{e}")
    })?;
    let mut res = Tags::new();
    if let Some(element) = html.select(&selector).next() {
        res.state = element
            .select(&state_selector)
            .next()
            .map_or_else(|| String::new(), |s| s.text().collect::<String>());

        res.label = element
            .select(&label_selector)
            .map(|label| {
                label
                    .text()
                    .fold(String::new(), |mut s, t| {
                        s.push_str(t);
                        s
                    })
                    .trim()
                    .to_string()
            })
            .collect();

        res.span = element
            .select(&span_selector)
            .map(|span| {
                span.text()
                    .fold(String::new(), |mut s, t| {
                        s.push_str(t);
                        s
                    })
                    .trim()
                    .to_string()
            })
            .collect()
    }
    Ok(res)
}

///提取所有的章节
pub fn extract_volume(html: &str, selector: &str) -> Result<Vec<Novel>> {
    let html = Html::parse_document(html);
    let volume_selector = Selector::parse("a").map_err(|e| {
        error!("{}", e);
        anyhow!("{e}")
    })?;
    let selector = Selector::parse(selector).map_err(|e| {
        error!("{}", e);
        anyhow!("{e}")
    })?;
    let mut res = Vec::new();
    if let Some(element) = html.select(&selector).next() {
        res = element
            .select(&volume_selector)
            .filter_map(|e| {
                let mut url = e.value().attr("href").unwrap_or_default().to_string();
                if let Ok(full_url) = Url::parse(&url) {
                    url = full_url.to_string();
                } else if Url::parse(&format!("https://www.linovelib.com{}", url)).is_ok() {
                    url = format!("https://www.linovelib.com{}", url);
                }

                let name = e.value().attr("title").unwrap_or_default().to_string();
                Some(Novel::new(url, name))
            })
            .collect::<Vec<_>>();
        res.reverse();
    }
    Ok(res)
}
