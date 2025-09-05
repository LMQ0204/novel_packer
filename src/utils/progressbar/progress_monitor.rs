// use crossterm::style::Stylize;
// progress.rs
use indicatif::{ProgressBar, ProgressStyle};
use std::{sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
}, time::Duration};

/// 进度监控器
#[derive(Clone)]
pub struct ProgressMonitor {
    pub pb: Arc<ProgressBar>,
    pub completed: Arc<AtomicUsize>,
    pub errors: Arc<AtomicUsize>,
    pub total: usize,
    pub prefix: String,
}

impl ProgressMonitor {
    /// 创建一个新的进度监控器
    pub fn new(total: usize, prefix: &str) -> Self {
        let pb = ProgressBar::new(total as u64);

        // 设置模板，将前缀放在最前面
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{prefix:.cyan} {spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );

        pb.set_prefix(prefix.to_owned()); // 设置前缀
        pb.enable_steady_tick(Duration::from_millis(100)); // 启用定期刷新

        Self {
            pb: Arc::new(pb),
            completed: Arc::new(AtomicUsize::new(0)),
            errors: Arc::new(AtomicUsize::new(0)),
            total,
            prefix: prefix.to_string(),
        }
    }

    /// 设置前缀
    pub fn set_title(&mut self, prefix: &str) {
        self.pb.set_prefix(prefix.to_owned());
        self.prefix = prefix.to_string();
    }

    /// 获取前缀
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// 获取完成百分比
    pub fn percentage(&self) -> f32 {
        let completed = self.completed.load(Ordering::Relaxed);
        (completed as f32 / self.total as f32) * 100.0
    }

    /// 获取完成数量
    pub fn completed_count(&self) -> usize {
        self.completed.load(Ordering::Relaxed)
    }

    /// 增加完成计数
    pub fn increment(&self) {
        let prev = self.completed.fetch_add(1, Ordering::Relaxed);
        self.pb.set_position((prev + 1) as u64);
        self.update_message();
    }

    /// 记录一个错误
    pub fn record_error(&self) {
        let prev = self.completed.fetch_add(1, Ordering::Relaxed);
        self.pb.set_position((prev + 1) as u64);
        self.errors.fetch_add(1, Ordering::Relaxed);
        self.update_message();
    }

    /// 获取错误数量
    pub fn error_count(&self) -> usize {
        self.errors.load(Ordering::Relaxed)
    }

    /// 更新进度条消息
    fn update_message(&self) {
        let errors = self.error_count();
        if errors > 0 {
            self.pb.set_message(format!("下载中…… ({} 个错误)", errors));
        } else {
            self.pb
                .set_message("下载中…… ");
        }
    }

    /// 完成进度监控，显示错误统计，但不消失
    pub fn finish(&self) {
        let errors = self.error_count();
        
        // 设置最终消息，但不调用finish_with_message，这样进度条不会消失
        if errors > 0 {
            self.pb.set_message(format!("完成! ({} 个错误)", errors));
        } else {
            self.pb.set_message("完成!");
        }
        
        // 确保进度条显示100%
        self.pb.set_position(self.total as u64);
        
        // 禁用自动刷新，但保持显示
        self.pb.disable_steady_tick();
        
        // 输出统计信息
        println!();
        // println!("任务完成: {}/{} ({}%)", 
        //     self.completed_count(), 
        //     self.total, 
        //     self.percentage()
        // );
        // if errors > 0 {
        //     println!("错误数: {}", errors);
        // }
    }

    /// 强制完成并隐藏进度条（如果需要）
    pub fn finish_and_hide(&self) {
        let errors = self.error_count();
        if errors > 0 {
            self.pb
                .finish_with_message(format!("所有章节下载完成 ({} 个错误)", errors));
        } else {
            self.pb.finish_with_message("所有章节下载完成");
        }
    }
}