//! Rust Controller Library
//!
//! 一个简单灵活的 HTTP 服务器控制库，支持配置管理和文件上传功能。

mod config;
mod controller;
mod multipart;
mod server; // 修改：将upload改为multipart

pub use config::{AppConfig, ImageData};
pub use controller::Controller;

// 使用 anyhow 作为错误处理库
pub use anyhow::Result;

use once_cell::sync::OnceCell;
use std::{collections::HashMap, sync::Mutex};

// 全局控制器实例
static CONTROLLER: OnceCell<Mutex<Option<Controller>>> = OnceCell::new();

/// 初始化全局控制器
pub fn init_controller(config: AppConfig) -> Result<()> {
    let controller = Controller::new(config);
    CONTROLLER
        .set(Mutex::new(Some(controller)))
        .map_err(|_| anyhow::anyhow!("Controller already initialized"))
}

/// 获取全局控制器引用
pub fn get_controller() -> Result<std::sync::MutexGuard<'static, Option<Controller>>> {
    CONTROLLER
        .get()
        .ok_or_else(|| anyhow::anyhow!("Controller not initialized"))
        .map(|cell| cell.lock().unwrap())
}

/// 启动服务器
pub fn start_server() -> Result<()> {
    let mut controller_guard = get_controller()?;
    if let Some(controller) = controller_guard.as_mut() {
        controller.start_server()
    } else {
        Err(anyhow::anyhow!("Controller not available"))
    }
}

/// 停止服务器
pub fn stop_server() -> Result<()> {
    let mut controller_guard = get_controller()?;
    if let Some(controller) = controller_guard.as_mut() {
        controller.stop_server()
    } else {
        Err(anyhow::anyhow!("Controller not available"))
    }
}

/// 通过URL获取图片数据
pub fn get_image_by_url(url: &str) -> Result<Option<ImageData>> {
    let controller_guard = get_controller()?;
    if let Some(controller) = controller_guard.as_ref() {
        Ok(controller.get_image_by_url(url))
    } else {
        Err(anyhow::anyhow!("Controller not available"))
    }
}

/// 获取所有图片数据
pub fn get_all_images() -> Result<std::collections::HashMap<String, ImageData>> {
    let controller_guard = get_controller()?;
    if let Some(controller) = controller_guard.as_ref() {
        Ok(controller.get_all_images())
    } else {
        Err(anyhow::anyhow!("Controller not available"))
    }
}

/// 清空所有图片数据
pub fn clear_images() -> Result<()> {
    let controller_guard = get_controller()?;
    if let Some(controller) = controller_guard.as_ref() {
        controller.clear_images();
        Ok(())
    } else {
        Err(anyhow::anyhow!("Controller not available"))
    }
}

/// 移除特定URL的图片数据
pub fn remove_image(url: &str) -> Result<Option<ImageData>> {
    let controller_guard = get_controller()?;
    if let Some(controller) = controller_guard.as_ref() {
        Ok(controller.remove_image(url))
    } else {
        Err(anyhow::anyhow!("Controller not available"))
    }
}

/// 检查服务器是否正在运行
pub fn is_server_running() -> Result<bool> {
    let controller_guard = get_controller()?;
    if let Some(controller) = controller_guard.as_ref() {
        Ok(controller.is_running())
    } else {
        Err(anyhow::anyhow!("Controller not available"))
    }
}

/// 获取服务器端口
pub fn get_server_port() -> Result<u16> {
    let controller_guard = get_controller()?;
    if let Some(controller) = controller_guard.as_ref() {
        Ok(controller.get_port())
    } else {
        Err(anyhow::anyhow!("Controller not available"))
    }
}

pub fn update_config<F>(updater: F) -> Result<()>
where
    F: FnOnce(&mut AppConfig),
{
    let controller_guard = get_controller()?;
    if let Some(controller) = controller_guard.as_ref() {
        controller.update_config(updater)
    } else {
        Err(anyhow::anyhow!("Controller not available"))
    }
}

/// 设置整个图片HashMap
pub fn set_images(new_images: HashMap<String, ImageData>) -> Result<()> {
    let controller_guard = get_controller()?;
    if let Some(controller) = controller_guard.as_ref() {
        controller.set_images(new_images);
        Ok(())
    } else {
        Err(anyhow::anyhow!("Controller not available"))
    }
}

/// 批量添加或更新图片
pub fn add_or_update_images(images: HashMap<String, ImageData>) -> Result<()> {
    let controller_guard = get_controller()?;
    if let Some(controller) = controller_guard.as_ref() {
        controller.add_or_update_images(images);
        Ok(())
    } else {
        Err(anyhow::anyhow!("Controller not available"))
    }
}

/// 从文件加载图片数据
pub fn load_images_from_file(filename: &str) -> Result<()> {
    let controller_guard = get_controller()?;
    if let Some(controller) = controller_guard.as_ref() {
        controller.load_images_from_file(filename)
    } else {
        Err(anyhow::anyhow!("Controller not available"))
    }
}

/// 保存图片数据到文件
pub fn save_images_to_file(filename: &str, compression_level: u32) -> Result<()> {
    let controller_guard = get_controller()?;
    if let Some(controller) = controller_guard.as_ref() {
        controller.save_images_to_file(filename, compression_level)
    } else {
        Err(anyhow::anyhow!("Controller not available"))
    }
}

/// 将图片数据保存为JSON格式的文本文件
pub fn save_images_to_json_file(filename: &str, pretty: bool) -> Result<()> {
    let controller_guard = get_controller()?;
    if let Some(controller) = controller_guard.as_ref() {
        controller.save_images_to_json_file(filename, pretty)
    } else {
        Err(anyhow::anyhow!("Controller not available"))
    }
}

/// 过滤图片数据
pub fn filter_images<F>(predicate: F) -> Result<HashMap<String, ImageData>>
where
    F: Fn(&String, &ImageData) -> bool,
{
    let controller_guard = get_controller()?;
    if let Some(controller) = controller_guard.as_ref() {
        Ok(controller.filter_images(predicate))
    } else {
        Err(anyhow::anyhow!("Controller not available"))
    }
}