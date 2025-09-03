use scraper::{Html, Selector};
use anyhow::{Result, anyhow};
use tracing::error;

/// 删除匹配选择器的所有元素
pub fn remove_elements(html: &str, selector: &str) -> Result<String> {
    let document = Html::parse_document(html);
    let selector = Selector::parse(selector).map_err(|e| {
        error!("{}", e);
        anyhow!("{e}")
    })?;
    let mut result = document.html();
    
    for element in document.select(&selector) {
        let element_html = element.html();
        result = result.replace(&element_html, "");
    }
    
    Ok(result)
}

/// 删除匹配选择器的第一个元素
pub fn remove_first_element(html: &str, selector: &str) -> Result<String> {
    let document = Html::parse_document(html);
    let selector = Selector::parse(selector).map_err(|e| {
        error!("{}", e);
        anyhow!("{e}")
    })?;
    let mut result = document.html();
    
    if let Some(element) = document.select(&selector).next() {
        let element_html = element.html();
        result = result.replace(&element_html, "");
    }
    
    Ok(result)
}

/// 修改元素的属性
pub fn set_attributes(html: &str, selector: &str, attributes: &[(&str, &str)]) -> Result<String> {
    let document = Html::parse_document(html);
    let selector = Selector::parse(selector).map_err(|e| {
        error!("{}", e);
        anyhow!("{e}")
    })?;
    let mut result = document.html();
    
    for element in document.select(&selector) {
        let old_html = element.html();
        let tag_name = element.value().name();
        let mut new_html = String::new();
        
        // 构建开始标签
        new_html.push('<');
        new_html.push_str(tag_name);
        
        // 添加现有属性
        for (name, value) in element.value().attrs() {
            // 检查是否需要覆盖此属性
            let should_override = attributes.iter().any(|(n, _)| *n == name);
            if !should_override {
                new_html.push(' ');
                new_html.push_str(name);
                new_html.push_str("=\"");
                new_html.push_str(value);
                new_html.push('"');
            }
        }
        
        // 添加新属性
        for (name, value) in attributes {
            new_html.push(' ');
            new_html.push_str(name);
            new_html.push_str("=\"");
            new_html.push_str(value);
            new_html.push('"');
        }
        
        new_html.push('>');
        
        // 添加内容
        new_html.push_str(&element.inner_html());
        
        // 添加结束标签
        new_html.push_str("</");
        new_html.push_str(tag_name);
        new_html.push('>');
        
        // 替换 HTML
        result = result.replace(&old_html, &new_html);
    }
    
    Ok(result)
}

/// 修改元素的文本内容
pub fn set_text(html: &str, selector: &str, text: &str) -> Result<String> {
    let document = Html::parse_document(html);
    let selector = Selector::parse(selector).map_err(|e| {
        error!("{}", e);
        anyhow!("{e}")
    })?;
    let mut result = document.html();
    
    for element in document.select(&selector) {
        let old_html = element.html();
        let tag_name = element.value().name();
        let mut new_html = String::new();
        
        // 构建开始标签
        new_html.push('<');
        new_html.push_str(tag_name);
        
        // 添加属性
        for (name, value) in element.value().attrs() {
            new_html.push(' ');
            new_html.push_str(name);
            new_html.push_str("=\"");
            new_html.push_str(value);
            new_html.push('"');
        }
        
        new_html.push('>');
        
        // 添加新文本内容
        new_html.push_str(text);
        
        // 添加结束标签
        new_html.push_str("</");
        new_html.push_str(tag_name);
        new_html.push('>');
        
        // 替换 HTML
        result = result.replace(&old_html, &new_html);
    }
    
    Ok(result)
}

/// 在匹配的元素前插入 HTML
pub fn insert_before(html: &str, selector: &str, new_html: &str) -> Result<String> {
    let document = Html::parse_document(html);
    let selector = Selector::parse(selector).map_err(|e| {
        error!("{}", e);
        anyhow!("{e}")
    })?;
    let mut result = document.html();
    
    for element in document.select(&selector) {
        let element_html = element.html();
        let inserted_html = format!("{}{}", new_html, element_html);
        result = result.replace(&element_html, &inserted_html);
    }
    
    Ok(result)
}

/// 在匹配的元素后插入 HTML
pub fn insert_after(html: &str, selector: &str, new_html: &str) -> Result<String> {
    let document = Html::parse_document(html);
    let selector = Selector::parse(selector).map_err(|e| {
        error!("{}", e);
        anyhow!("{e}")
    })?;
    let mut result = document.html();
    
    for element in document.select(&selector) {
        let element_html = element.html();
        let inserted_html = format!("{}{}", element_html, new_html);
        result = result.replace(&element_html, &inserted_html);
    }
    
    Ok(result)
}

/// 在匹配的元素内部开头插入 HTML
pub fn prepend(html: &str, selector: &str, new_html: &str) -> Result<String> {
    let document = Html::parse_document(html);
    let selector = Selector::parse(selector).map_err(|e| {
        error!("{}", e);
        anyhow!("{e}")
    })?;
    let mut result = document.html();
    
    for element in document.select(&selector) {
        let old_html = element.html();
        let inner_html = element.inner_html();
        let tag_name = element.value().name();
        
        // 构建属性字符串
        let mut attrs = String::new();
        for (name, value) in element.value().attrs() {
            attrs.push(' ');
            attrs.push_str(name);
            attrs.push_str("=\"");
            attrs.push_str(value);
            attrs.push('"');
        }
        
        // 构建新 HTML
        let new_element_html = format!("<{}{}>{}{}</{}>", 
            tag_name, attrs, new_html, inner_html, tag_name
        );
        
        // 替换 HTML
        result = result.replace(&old_html, &new_element_html);
    }
    
    Ok(result)
}

/// 在匹配的元素内部末尾插入 HTML
pub fn append(html: &str, selector: &str, new_html: &str) -> Result<String> {
    let document = Html::parse_document(html);
    let selector = Selector::parse(selector).map_err(|e| {
        error!("{}", e);
        anyhow!("{e}")
    })?;
    let mut result = document.html();
    
    for element in document.select(&selector) {
        let old_html = element.html();
        let inner_html = element.inner_html();
        let tag_name = element.value().name();
        
        // 构建属性字符串
        let mut attrs = String::new();
        for (name, value) in element.value().attrs() {
            attrs.push(' ');
            attrs.push_str(name);
            attrs.push_str("=\"");
            attrs.push_str(value);
            attrs.push('"');
        }
        
        // 构建新 HTML
        let new_element_html = format!("<{}{}>{}{}</{}>", 
            tag_name, attrs, inner_html, new_html, tag_name
        );
        
        // 替换 HTML
        result = result.replace(&old_html, &new_element_html);
    }
    
    Ok(result)
}

/// 提取匹配选择器的元素的文本内容
pub fn extract_text(html: &str, selector: &str) -> Result<Vec<String>> {
    let document = Html::parse_document(html);
    let selector = Selector::parse(selector).map_err(|e| {
        error!("{}", e);
        anyhow!("{e}")
    })?;
    
    let mut results = Vec::new();
    for element in document.select(&selector) {
        results.push(element.text().collect::<String>());
    }
    
    Ok(results)
}

/// 提取匹配选择器的元素的 HTML 内容
pub fn extract_html(html: &str, selector: &str) -> Result<Vec<String>> {
    let document = Html::parse_document(html);
    let selector = Selector::parse(selector).map_err(|e| {
        error!("{}", e);
        anyhow!("{e}")
    })?;
    
    let mut results = Vec::new();
    for element in document.select(&selector) {
        results.push(element.html());
    }
    
    Ok(results)
}

/// 检查是否存在匹配选择器的元素
pub fn exists(html: &str, selector: &str) -> Result<bool> {
    let document = Html::parse_document(html);
    let selector = Selector::parse(selector).map_err(|e| {
        error!("{}", e);
        anyhow!("{e}")
    })?;
    
    Ok(document.select(&selector).next().is_some())
}

/// 获取匹配选择器的元素数量
pub fn count(html: &str, selector: &str) -> Result<usize> {
    let document = Html::parse_document(html);
    let selector = Selector::parse(selector).map_err(|e| {
        error!("{}", e);
        anyhow!("{e}")
    })?;
    
    Ok(document.select(&selector).count())
}

/// 批量执行多个 HTML 处理操作
pub fn batch_process(
    html: &str, 
    operations: &[(&str, &str, Option<Vec<(&str, &str)>>, Option<&str>)]
) -> Result<String> {
    let mut result = html.to_string();
    
    for (operation, selector, attributes, content) in operations {
        match *operation {
            "remove" => {
                result = remove_elements(&result, selector)?;
            }
            "set_attributes" => {
                if let Some(attrs) = attributes {
                    result = set_attributes(&result, selector, attrs)?;
                }
            }
            "set_text" => {
                if let Some(text) = content {
                    result = set_text(&result, selector, text)?;
                }
            }
            "insert_before" => {
                if let Some(content) = content {
                    result = insert_before(&result, selector, content)?;
                }
            }
            "insert_after" => {
                if let Some(content) = content {
                    result = insert_after(&result, selector, content)?;
                }
            }
            "prepend" => {
                if let Some(content) = content {
                    result = prepend(&result, selector, content)?;
                }
            }
            "append" => {
                if let Some(content) = content {
                    result = append(&result, selector, content)?;
                }
            }
            _ => {
                return Err(anyhow!("未知的操作: {}", operation));
            }
        }
    }
    
    Ok(result)
}