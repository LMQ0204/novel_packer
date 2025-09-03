use crate::core::{init::URL_HANDLERS, singlefile::Singlefile};
use crate::utils::terminal::clear_previous_line;
use anyhow::{Result, anyhow};
use url::Url;

// 获取url对应的结构体
pub fn get_struct_by_url(url: &str) -> Result<Box<dyn Singlefile>> {
    let parsed_url = Url::parse(url).map_err(|e| anyhow!("URL解析失败: {}", e))?;

    let host = parsed_url
        .host_str()
        .ok_or_else(|| anyhow!("无法获取主机名: {}", url))?;

    let handlers = URL_HANDLERS.lock().unwrap();

    if let Some(handler) = handlers.get(host) {
        return Ok(handler(url.to_string()));
    }

    // 返回错误而不是默认值
    Err(anyhow!("不支持的网站: {}", host))
}

//从输入读取url
pub fn read_url_from_stdin() -> String {
    // let mut stdout = std::io::stdout();
    // println!("\n请输入URL(回车确定)[目前只支持哔哩轻小说]:");
    let use_input = loop {
        println!("请输入URL(回车确定)[目前只支持哔哩轻小说]:");
        let mut input = String::new();
        match std::io::stdin().read_line(&mut input) {
            Ok(_bytes_read) => {
                // 读取成功，返回去除换行符的 String（用 filter 排除空输入）
                let trimmed = input.trim().to_string();
                if trimmed.is_empty() {
                    clear_previous_line(3).unwrap_or_else(|e| eprintln!("清除屏幕失败：{}", e));

                    println!("警告：输入不能为空！");
                } else {
                    break trimmed;
                }
            }
            Err(e) => {
                clear_previous_line(3).unwrap_or_else(|e| eprintln!("清除屏幕失败：{}", e));
                // 读取失败，打印错误详情（而非终止程序）
                println!("读取输入出错：{}", e);
            }
        }
       
    };
    use_input
}

//返回url对应的结构体
pub fn get_from_url() -> Box<dyn Singlefile> {
    // let mut stdout = std::io::stdout();
    println!("\n");
    let res = loop {
        let input = read_url_from_stdin();
        match get_struct_by_url(&input) {
            Ok(s) => {
                //清除屏幕
                // clear_previous_line(1).unwrap_or_else(|e| eprintln!("清除屏幕失败：{}", e));
                println!("\n加载数据中……\n");
                break s;
            }
            Err(e) => {
                // 清除屏幕
                clear_previous_line(3).unwrap_or_else(|e| eprintln!("清除屏幕失败：{}", e));
                println!("{e}");
            }
        }
    };
    res
}
