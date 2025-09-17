use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    terminal::disable_raw_mode,
};
use tracing::warn;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc};

// 用户命令枚举
#[derive(Debug, Clone)]
pub enum UserCommand {
    Pause,      // 暂停下载
    Resume,     // 继续下载
    Status,     // 查看状态
    Quit,       // 退出程序
    Other(char), // 其他按键
}

// 修改按键监听器创建函数
pub fn create_key_listener() -> (mpsc::Sender<()>, mpsc::Receiver<UserCommand>) {
    let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);
    let (cmd_tx, cmd_rx) = mpsc::channel(10);
    
    let running = Arc::new(AtomicBool::new(true));
    
    // 设置 Ctrl+C 处理器 - 立即退出
    if let Err(e) = ctrlc::set_handler(move || {
        println!("\n收到 Ctrl+C 信号，立即退出...");
        std::process::exit(0);
    }) {
        warn!("无法设置 Ctrl+C 处理器: {}", e);
    }
    
    tokio::spawn(async move {
        // 启用原始模式
        // if let Err(e) = enable_raw_mode() {
        //     eprintln!("无法启用原始模式: {}", e);
        //     return;
        // }
        
        while running.load(Ordering::Relaxed) {
            // 检查是否收到停止信号
            if let Ok(()) = stop_rx.try_recv() {
                running.store(false, Ordering::Relaxed);
                break;
            }
            
            // 使用非阻塞方式检查按键事件
            if let Ok(true) = event::poll(std::time::Duration::from_millis(100)) {
                if let Event::Key(KeyEvent { code, modifiers: _, .. }) = event::read().unwrap() {
                    let command = match code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => UserCommand::Quit,
                        KeyCode::Char('p') | KeyCode::Char('P') => UserCommand::Pause,
                        KeyCode::Char('r') | KeyCode::Char('R') => UserCommand::Resume,
                        KeyCode::Char('s') | KeyCode::Char('S') => UserCommand::Status,
                        KeyCode::Char(c) => UserCommand::Other(c),
                        _ => continue, // 忽略其他按键
                    };
                    
                    // 发送命令
                    if cmd_tx.send(command).await.is_err() {
                        break; // 接收端已断开
                    }
                }
            }
            
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        
        // 禁用原始模式
        if let Err(e) = disable_raw_mode() {
            eprintln!("无法禁用原始模式: {}", e);
        }
    });
    
    (stop_tx, cmd_rx)
}