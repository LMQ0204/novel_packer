use anyhow::{Result, anyhow};

use crate:: utils::check_single_file;
use async_trait::async_trait; // 导入宏
#[async_trait]
pub trait Singlefile: Send + Sync {
    fn check(&self) -> Result<()> {
        check_single_file::check_exe("./extra/single-file.exe")
            .map_err(|s| anyhow!("{}", s))         
    }
    async fn display(&mut self) -> Result<()>;
    async fn download(&mut self) -> Result<()>;
    // fn get_novel(&self) -> Result<()>;

}

