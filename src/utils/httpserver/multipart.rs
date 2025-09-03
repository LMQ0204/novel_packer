// 这个文件保持不变，和之前的upload.rs功能相同
use std::io::Read;
use anyhow::{Result, Context};

/// 解析后的multipart数据
pub struct ParsedMultipart {
    pub filename: Option<String>,
    pub url: Option<String>,
    pub mime_type: Option<String>,
    pub file_data: Option<Vec<u8>>,
}

/// Multipart 解析器
pub struct MultipartParser;

impl MultipartParser {
    /// 解析multipart数据
    pub fn parse_multipart(
        reader: &mut dyn Read,
        content_type: &str,
    ) -> Result<ParsedMultipart> {
        // 从 Content-Type 中提取 boundary
        let boundary = Self::extract_boundary(content_type)
            .context("No boundary found in Content-Type header")?;
        
        // 读取整个请求体
        let mut body = Vec::new();
        reader.read_to_end(&mut body)
            .context("Failed to read request body")?;
        
        // 初始化结果
        let mut result = ParsedMultipart {
            filename: None,
            url: None,
            mime_type: None,
            file_data: None,
        };
        
        // 分割 multipart 数据
        let parts = Self::split_multipart(&body, &boundary);
        
        // 处理每个部分
        for part in parts {
            let part_str = String::from_utf8_lossy(&part);
            
            // 检查内容类型和内容处置
            if let Some(name) = Self::extract_field_name(&part_str) {
                match name.as_str() {
                    "file" => {
                        result.filename = Self::extract_filename(&part_str);
                        result.file_data = Some(Self::extract_file_content(&part));
                    },
                    "filename" => {
                        result.filename = Some(Self::extract_field_value(&part));
                    },
                    "url" => {
                        result.url = Some(Self::extract_field_value(&part));
                    },
                    "mimeType" => {
                        result.mime_type = Some(Self::extract_field_value(&part));
                    },
                    _ => {}
                }
            }
        }
        
        Ok(result)
    }
    
    /// 从 Content-Type 字符串中提取 boundary
    fn extract_boundary(content_type: &str) -> Option<String> {
        content_type
            .find("boundary=")
            .map(|pos| {
                let boundary = &content_type[pos + 9..]; // "boundary=" 的长度是 9
                // 去除可能的引号
                boundary.trim_matches('"').trim().to_string()
            })
    }
    
    /// 分割 multipart 数据
    fn split_multipart(data: &[u8], boundary: &str) -> Vec<Vec<u8>> {
        let boundary_pattern = format!("\r\n--{}", boundary).into_bytes();
        let mut parts = Vec::new();
        let mut start = 0;
        
        while let Some(pos) = Self::find_subsequence(&data[start..], &boundary_pattern) {
            if pos > 0 {
                // 减去 2 是为了去除前面的 \r\n
                let part_end = start + pos;
                parts.push(data[start..part_end].to_vec());
            }
            start += pos + boundary_pattern.len();
            
            // 检查是否到达结束边界
            if start + 2 <= data.len() && &data[start..start + 2] == b"--" {
                break;
            }
        }
        
        parts
    }
    
    /// 在字节数组中查找子序列
    fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack.windows(needle.len()).position(|window| window == needle)
    }
    
    /// 从 multipart 部分中提取字段名
    fn extract_field_name(part: &str) -> Option<String> {
        if let Some(start) = part.find("name=\"") {
            let start = start + 6; // "name=\"" 的长度是 6
            if let Some(end) = part[start..].find('\"') {
                return Some(part[start..start + end].to_string());
            }
        }
        None
    }
    
    /// 从 multipart 部分中提取字段值
    fn extract_field_value(part: &[u8]) -> String {
        let part_str = String::from_utf8_lossy(part);
        
        // 查找内容分隔符
        if let Some(pos) = part_str.find("\r\n\r\n") {
            return part_str[pos + 4..].trim().to_string(); // +4 跳过 \r\n\r\n
        }
        
        String::new()
    }
    
    /// 从 multipart 部分中提取文件名
    fn extract_filename(part: &str) -> Option<String> {
        // 查找 filename= 或 filename="
        if let Some(start) = part.find("filename=\"") {
            let start = start + 10; // "filename=\"" 的长度是 10
            if let Some(end) = part[start..].find('\"') {
                return Some(part[start..start + end].to_string());
            }
        } else if let Some(start) = part.find("filename=") {
            let start = start + 9; // "filename=" 的长度是 9
            // 查找行尾或分号
            let end = part[start..]
                .find("\r\n")
                .unwrap_or(part[start..].len());
            
            let filename = &part[start..start + end];
            // 去除可能的引号
            return Some(filename.trim_matches('"').to_string());
        }
        
        None
    }
    
    /// 从 multipart 部分中提取文件内容
    fn extract_file_content(part: &[u8]) -> Vec<u8> {
        let part_str = String::from_utf8_lossy(part);
        
        // 查找内容分隔符
        if let Some(pos) = part_str.find("\r\n\r\n") {
            return part[pos + 4..].to_vec(); // +4 跳过 \r\n\r\n
        }
        
        Vec::new()
    }
}