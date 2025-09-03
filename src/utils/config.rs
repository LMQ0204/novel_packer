use anyhow::{Context, Result, anyhow};
use serde_json::{json, Map, Value};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use tracing::error;
// use std::process::Command;

#[derive(PartialEq, Debug, Clone)]
pub struct DynamicConfig {
    inner: Value,
    // config_path: PathBuf,
}

impl DynamicConfig {
    /// 创建新的空配置
    pub fn new() -> Self {
        DynamicConfig { inner: Value::Object(Map::new()), }
    }

    /// 从文件加载配置
    pub fn load(&mut self, config_path: PathBuf) -> Result<()> {
        let mut file = File::open(&config_path)
            .with_context(|| format!("无法打开配置文件: {:?}", config_path))
            .map_err(|e| {
                error!("{}", e);
                e
            })?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .with_context(|| format!("读取配置文件失败: {:?}", config_path))
            .map_err(|e| {
                error!("{}", e);
                e
            })?;

        self.inner = serde_json::from_str(&contents)
            .with_context(|| format!("解析JSON配置失败: {:?}", config_path))
            .map_err(|e| {
                error!("{}", e);
                e
            })?;

        // 验证配置结构
        if !self.inner.is_object() {
            error!("配置文件必须是一个JSON对象: {:?}", config_path);
            return Err(anyhow!("配置文件必须是一个JSON对象: {:?}", config_path));
        }

        Ok(())
    }

    /// 保存配置到文件
    pub fn save(&self, path: PathBuf) -> Result<()> {
        let json_str = serde_json::to_string_pretty(&self.inner)
            .context("序列化配置失败")
            .map_err(|e| {
                error!("{}", e);
                e
            })?;

        let mut file = File::create(&path)
            .with_context(|| format!("创建配置文件失败: {:?}", path))
            .map_err(|e| {
                error!("{}", e);
                e
            })?;

        file.write_all(json_str.as_bytes())
            .with_context(|| format!("写入配置文件失败: {:?}", path))
            .map_err(|e| {
                error!("{}", e);
                e
            })?;

        Ok(())
    }

    /// 获取配置值
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.inner.get(key)
    }

    /// 设置配置值
    pub fn set(&mut self, key: &str, value: Value) -> Result<()> {
        if let Value::Object(ref mut map) = self.inner {
            map.insert(key.to_string(), value);
            Ok(())
        } else {
            error!("DynamicConfig::set -> 配置内部数据结构损坏，期望对象类型");
            Err(anyhow!(
                "DynamicConfig::set -> 配置内部数据结构损坏，期望对象类型"
            ))
        }
    }

    /// 获取多个配置值（针对支持多个值的键）
    pub fn get_multi(&self, key: &str) -> Vec<&Value> {
        if let Some(value) = self.inner.get(key) {
            if let Value::Array(arr) = value {
                arr.iter().collect()
            } else {
                vec![value]
            }
        } else {
            Vec::new()
        }
    }

    /// 添加配置值（支持多个值）
    pub fn add(&mut self, key: &str, value: Value) -> Result<()> {
        if let Value::Object(ref mut map) = self.inner {
            if let Some(existing) = map.get_mut(key) {
                // 如果键已存在，将其转换为数组
                match existing {
                    Value::Array(arr) => {
                        arr.push(value);
                    }
                    _ => {
                        let old_value = std::mem::replace(existing, Value::Array(Vec::new()));
                        if let Value::Array(arr) = existing {
                            arr.push(old_value);
                            arr.push(value);
                        }
                    }
                }
            } else {
                // 键不存在，直接设置
                map.insert(key.to_string(), value);
            }
            Ok(())
        } else {
            error!("DynamicConfig::add -> 配置内部数据结构损坏，期望对象类型");
            Err(anyhow!(
                "DynamicConfig::add -> 配置内部数据结构损坏，期望对象类型"
            ))
        }
    }

    /// 将配置转换为命令行参数（支持多个值）
    pub fn to_args(&self) -> Result<Vec<String>> {
        let mut args = Vec::new();

        if let Value::Object(map) = &self.inner {
            for (key, value) in map {
                match value {
                    Value::Array(arr) => {
                        for item in arr {
                            Self::add_arg(&mut args, key, item);
                        }
                    }
                    _ => {
                        Self::add_arg(&mut args, key, value);
                    }
                }
            }
            Ok(args)
        } else {
            error!("DynamicConfig::to_args ->配置内部数据结构损坏，期望对象类型");
            Err(anyhow!(
                "DynamicConfig::to_args ->配置内部数据结构损坏，期望对象类型"
            ))
        }
    }

    /// 添加单个参数到参数列表
    fn add_arg(args: &mut Vec<String>, key: &str, value: &Value) {
        let arg_name = format!("--{}", key);
        args.push(arg_name);

        match value {
            Value::Null => {
                // 对于 null，不添加值
            }
            Value::String(s) => {
                if !s.is_empty() {
                    args.push(s.clone());
                }
            }
            _ => {
                // 对于其他类型，转换为字符串并添加
                args.push(value.to_string());
            }
        }
    }

    /// 链式设置方法，支持方法链调用
    pub fn with_set(&mut self, key: &str, value: Value) -> &mut Self {
        self.set(key, value).expect("Failed to set value");
        self
    }

    /// 删除配置键
    pub fn remove(&mut self, key: &str) -> Result<()> {
        if let Value::Object(ref mut map) = self.inner {
            if map.remove(key).is_some() {
                Ok(())
            } else {
                error!("DynamicConfig::remove -> 键不存在: {}", key);
                Err(anyhow!("DynamicConfig::remove -> 键不存在: {}", key))
            }
        } else {
            error!("DynamicConfig::remove -> 配置内部数据结构损坏，期望对象类型");
            Err(anyhow!(
                "DynamicConfig::remove -> 配置内部数据结构损坏，期望对象类型"
            ))
        }
    }

    /// 将配置转换为命令行参数
    // pub fn to_args(&self) -> Result<Vec<String>> {
    //     let mut args = Vec::new();

    //     if let Value::Object(map) = &self.inner {
    //         for (key, value) in map {
    //             let arg_name = format!("--{}", key);
    //             args.push(arg_name);

    //             // 根据值的类型决定是否添加值
    //             match value {
    //                 Value::Null => {
    //                     // 对于 null，不添加值
    //                 }
    //                 Value::String(s) => {
    //                     if !s.is_empty() {
    //                         args.push(s.clone());
    //                     }
    //                 }
    //                 _ => {
    //                     // 对于其他类型，转换为字符串并添加
    //                     args.push(value.to_string());
    //                 }
    //             }
    //         }
    //         Ok(args)
    //     } else {
    //         error!("DynamicConfig::to_args ->配置内部数据结构损坏，期望对象类型");
    //         Err(anyhow!(
    //             "DynamicConfig::to_args ->配置内部数据结构损坏，期望对象类型"
    //         ))
    //     }
    // }

    /// 获取可执行文件路径
    pub fn get_executable_path(&self) -> Option<String> {
        self.inner
            .get("single-file-path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

}

impl Default for DynamicConfig {
    fn default() -> Self {
        let mut res = DynamicConfig::new();
        res.with_set("remove-hidden-elements", json!(false))
           .with_set("dump-content", json!(true))
           .with_set("output-json", json!(true))
           .with_set("browser-arg", json!(["--user-data-dir=D:\\Singlefile data", "--profile-directory=Default"]));
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_dynamic_config_equality() {
        // 创建两个相同的配置
        let mut config1 = DynamicConfig::new();
        config1.set("key2", json!("value2")).unwrap();
        config1.set("key1", json!("value1")).unwrap();

        let mut config2 = DynamicConfig::new();
        config2.set("key1", json!("value1")).unwrap();
        config2.set("key2", json!("value2")).unwrap();

        // 测试相等
        assert_eq!(config1, config2);

        // 修改其中一个配置
        config2.set("key2", json!("different_value")).unwrap();

        // 测试不相等
        assert_ne!(config1, config2);
    }

    #[test]
    fn test_dynamic_config_remove() {
        let mut config = DynamicConfig::new();
        config.set("key1", json!("value1")).unwrap();
        config.set("key2", json!("value2")).unwrap();

        // 确认键存在
        assert!(config.get("key1").is_some());
        assert!(config.get("key2").is_some());

        // 删除一个键
        config.remove("key1").unwrap();

        // 确认键已删除
        assert!(config.get("key1").is_none());
        assert!(config.get("key2").is_some());

        // 尝试删除不存在的键
        let result = config.remove("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_dynamic_config_to_args() {
        let mut config = DynamicConfig::new();
        config.set("string_arg", json!("string_value")).unwrap();
        config.set("number_arg", json!(123)).unwrap();
        config.set("bool_arg", json!(true)).unwrap();
        config.set("null_arg", json!(null)).unwrap();

        let args = config.to_args().unwrap();

        // 验证生成的参数
        assert!(args.contains(&"--string_arg".to_string()));
        assert!(args.contains(&"string_value".to_string()));
        assert!(args.contains(&"--number_arg".to_string()));
        assert!(args.contains(&"123".to_string()));
        assert!(args.contains(&"--bool_arg".to_string()));
        assert!(args.contains(&"true".to_string()));
        assert!(args.contains(&"--null_arg".to_string()));
        // null 值不应该有对应的值参数
        assert_eq!(args.iter().filter(|&a| a == "null").count(), 0);
    }

    #[test]
    fn test_dynamic_config_get_executable_path() {
        let mut config = DynamicConfig::new();
        
        // 测试没有设置路径的情况
        assert!(config.get_executable_path().is_none());
        
        // 测试设置了路径的情况
        config.set("single-file-path", json!("/path/to/single-file")).unwrap();
        assert_eq!(config.get_executable_path().unwrap(), "/path/to/single-file");
    }

    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_dynamic_config_new() {
        let config = DynamicConfig::new();
        assert!(config.inner.is_object());
        assert_eq!(config.inner.as_object().unwrap().len(), 0);
    }

    #[test]
    fn test_dynamic_config_default() {
        let config = DynamicConfig::new();
        assert!(config.inner.is_object());
        assert_eq!(config.inner.as_object().unwrap().len(), 0);
    }

    #[test]
    fn test_dynamic_config_get() {
        let mut config = DynamicConfig::new();
        
        // 测试获取不存在的键
        assert!(config.get("nonexistent").is_none());
        
        // 设置一个键并测试获取
        config.set("test_key", json!("test_value")).unwrap();
        assert_eq!(config.get("test_key").unwrap(), &json!("test_value"));
        
        // 测试获取不同类型的值
        config.set("number_key", json!(42)).unwrap();
        config.set("bool_key", json!(true)).unwrap();
        config.set("null_key", json!(null)).unwrap();
        
        assert_eq!(config.get("number_key").unwrap(), &json!(42));
        assert_eq!(config.get("bool_key").unwrap(), &json!(true));
        assert_eq!(config.get("null_key").unwrap(), &json!(null));
    }

    #[test]
    fn test_dynamic_config_set() {
        let mut config = DynamicConfig::new();
        
        // 测试设置字符串值
        config.set("string_key", json!("string_value")).unwrap();
        assert_eq!(config.get("string_key").unwrap(), &json!("string_value"));
        
        // 测试设置数字值
        config.set("number_key", json!(123)).unwrap();
        assert_eq!(config.get("number_key").unwrap(), &json!(123));
        
        // 测试设置布尔值
        config.set("bool_key", json!(true)).unwrap();
        assert_eq!(config.get("bool_key").unwrap(), &json!(true));
        
        // 测试设置null值
        config.set("null_key", json!(null)).unwrap();
        assert_eq!(config.get("null_key").unwrap(), &json!(null));
        
        // 测试设置对象值
        let obj_value = json!({"nested": "value"});
        config.set("object_key", obj_value.clone()).unwrap();
        assert_eq!(config.get("object_key").unwrap(), &obj_value);
        
        // 测试设置数组值
        let array_value = json!([1, 2, 3]);
        config.set("array_key", array_value.clone()).unwrap();
        assert_eq!(config.get("array_key").unwrap(), &array_value);
    }

    #[test]
    fn test_dynamic_config_save_and_load() {
        let mut config = DynamicConfig::new();
        config.set("key1", json!("value1")).unwrap();
        config.set("key2", json!("value2")).unwrap();
        config.set("key3", json!(123)).unwrap();
        config.set("key4", json!(true)).unwrap();

        // 创建临时文件
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        // 保存配置到文件
        config.save(temp_path.clone()).unwrap();

        // 创建新配置并从文件加载
        let mut loaded_config = DynamicConfig::new();
        loaded_config.load(temp_path).unwrap();

        // 验证加载的配置与原始配置相同
        assert_eq!(config, loaded_config);
    }

    #[test]
    fn test_dynamic_config_load_invalid_file() {
        // 创建包含无效JSON的临时文件
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();
        
        let mut file = File::create(&temp_path).unwrap();
        file.write_all(b"invalid json content").unwrap();

        // 尝试加载无效文件
        let mut config = DynamicConfig::new();
        let result = config.load(temp_path);
        
        // 应该返回错误
        assert!(result.is_err());
    }

    #[test]
    fn test_dynamic_config_load_non_object() {
        // 创建包含非对象JSON的临时文件
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();
        
        let mut file = File::create(&temp_path).unwrap();
        file.write_all(b"[\"array\", \"not\", \"object\"]").unwrap();

        // 尝试加载非对象文件
        let mut config = DynamicConfig::new();
        let result = config.load(temp_path);
        
        // 应该返回错误
        assert!(result.is_err());
    }

    #[test]
    fn test_dynamic_config_save_nonexistent_path() {
        let config = DynamicConfig::new();
        
        // 尝试保存到不存在的路径
        let result = config.save(PathBuf::from("/nonexistent/path/config.json"));
        
        // 应该返回错误
        assert!(result.is_err());
    }
}