use std::path::{PathBuf};
use anyhow::{Result, anyhow};
use tracing::warn;

use crate::utils::{check_single_file, config::DynamicConfig};
use async_trait::async_trait; // 导入宏
#[async_trait]
pub trait Singlefile: Send + Sync {
    fn check(&self) -> Result<()> {
        let mut config = DynamicConfig::new();
        config.load(PathBuf::from("config.json")).unwrap_or_else(|e| {
            warn!("{}",e);
        });
        check_single_file::check_exe("single-file.exe", config)
            .map_err(|s| anyhow!("{}", s))
            
    }
    fn init(&mut self) -> Result<()>;
    async fn display(&mut self) -> Result<()>;
    // fn download();
    // fn extract();
}

