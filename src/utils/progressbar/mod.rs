pub mod progress_monitor;
use std::{sync::{Arc, atomic::AtomicUsize}, time::Instant};

use indicatif::{ProgressBar, ProgressStyle};

use crate::utils::progressbar::progress_monitor::ProgressMonitor;

// progress.rs
pub struct ProgressMonitorBuilder {
    total: usize,
    template: Option<String>,
    progress_chars: Option<String>,
    initial_message: Option<String>,
    prefix: String
}

impl ProgressMonitorBuilder {
    pub fn new(total: usize) -> Self {
        Self {
            total,
            template: None,
            progress_chars: None,
            initial_message: None,
            prefix: String::new()
        }
    }
    
    pub fn template(mut self, template: &str) -> Self {
        self.template = Some(template.to_string());
        self
    }
    
    pub fn progress_chars(mut self, chars: &str) -> Self {
        self.progress_chars = Some(chars.to_string());
        self
    }
    
    pub fn initial_message(mut self, message: &str) -> Self {
        self.initial_message = Some(message.to_string());
        self
    }
    
    pub fn set_prefix(mut self, prefix: &str) -> Self {
        self.prefix = prefix.to_string();
        self
    }

    pub fn build(self) -> ProgressMonitor {
        let pb = ProgressBar::new(self.total as u64);
        
        let mut style = ProgressStyle::default_bar();
        
        if let Some(template) = self.template {
            style = style.template(&template).unwrap();
        }
        
        if let Some(chars) = self.progress_chars {
            style = style.progress_chars(&chars);
        }
        
        pb.set_style(style);
        
        if let Some(message) = self.initial_message {
            pb.set_message(message);
        } else {
            pb.set_message("处理中");
        }
        
        ProgressMonitor {
            pb: Arc::new(pb),
            completed: Arc::new(AtomicUsize::new(0)),
            errors: Arc::new(AtomicUsize::new(0)),
            total: self.total,
            prefix: self.prefix
        }
    }
}