use std::collections::HashSet;
use anyhow::{Result, anyhow};

/// 从用户输入提取数字（支持空格分隔和 a-b 范围格式），并自动去重
/// 输入示例："1 3-5 3 7-9 8" → 输出：1, 3, 4, 5, 7, 8, 9
pub fn get_index_from_stdin(input: &str) -> Result<Vec<u8>> {
    let mut result = HashSet::new(); // 使用HashSet自动去重

    for token in input.split_whitespace() {
        if token.contains('-') {
            // 处理范围格式（a-b）
            let parts: Vec<&str> = token.split('-').collect();
            if parts.len() != 2 {
                return Err(anyhow!("效的范围格式: {}", token));
            }

            let a = parts[0].parse::<u8>().map_err(|e| {
                anyhow!("范围起始不是有效数字 '{}': {}", parts[0], e)
            })?;
            let b = parts[1].parse::<u8>().map_err(|e| {
                anyhow!("范围结束不是有效数字 '{}': {}", parts[1], e)
            })?;

            if a > b {
                return Err(anyhow!("范围无效（起始 > 结束）: {}-{}", a, b));
            }

            // 添加范围内的所有数字（HashSet会自动去重）
            for num in a..=b {
                result.insert(num);
            }
        } else {
            // 处理单个数字
            let num = token.parse::<u8>().map_err(|e| {
                anyhow!("不是有效数字 '{}': {}", token, e)
            })?;
            result.insert(num); // 重复数字会被自动忽略
        }
    }

    // 将HashSet转换为Vec并排序（可选）
    let mut vec_result: Vec<u8> = result.into_iter().collect();
    vec_result.sort(); // 排序使结果更直观

    Ok(vec_result)
}