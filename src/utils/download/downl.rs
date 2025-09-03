pub mod down {
    use std::path::PathBuf;

    use crate::utils::config::DynamicConfig;
    use anyhow::{Context, Result, anyhow};
    // use serde_json::Deserializer;
    use serde_json::Value;
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
    use tokio::process::Command;
    use tracing::{error, info};
    use url::Url;

    pub async fn download_from_url(url: &str, options: DynamicConfig) -> Result<Vec<Value>> {
        let exe_path = if let Some(path) = options.get_executable_path() {
            info!("从配置中读取到路径:{}", &path);
            path
        } else {
            info!("使用默认路径:{}", "single-file");
            String::from("single-file")
        };

        let args = options.to_args().map_err(|e| anyhow!("{}", e))?;

        // 创建命令
        let mut cmd = Command::new(&exe_path);
        cmd.arg(url)
            .args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // 启动进程
        let mut child = cmd
            .spawn()
            .with_context(|| format!("启动异步命令失败: {} {:?}", exe_path, args))
            .map_err(|e| {
                error!("{}", e);
                anyhow!("{}", e)
            })?;

        // 获取标准输出和错误输出
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        // 创建读取器
        let stdout_reader = BufReader::new(stdout);
        let stderr_reader = BufReader::new(stderr);

        // 同时处理标准输出和标准错误
        let stdout_handle = tokio::spawn(process_stdout(stdout_reader));
        let stderr_handle = tokio::spawn(process_stderr(stderr_reader));

        // 等待进程完成
        let status = child.wait().await?;

        // 等待所有任务完成
        let (stdout_result, stderr_result) = tokio::join!(stdout_handle, stderr_handle);

        // 检查进程退出状态
        if !status.success() {
            error!("命令执行失败，退出状态: {}", status);
            return Err(anyhow!("命令执行失败，退出状态: {}", status));
        }

        // 检查标准错误处理结果
        if let Err(e) = stderr_result {
            error!("标准错误处理任务失败: {}", e);
            return Err(anyhow!("标准错误处理任务失败: {}", e));
        }

        // 返回标准输出处理结果（解析的JSON）
        match stdout_result {
            Ok(Ok(results)) => Ok(results),
            Ok(Err(e)) => {
                error!("标准输出处理失败: {}", e);
                Err(anyhow!("标准输出处理失败: {}", e))
            }
            Err(e) => {
                error!("标准输出处理任务失败: {}", e);
                Err(anyhow!("标准输出处理任务失败: {}", e))
            }
        }
    }
    pub async fn download_from_file(file: &str, options: DynamicConfig) -> Result<Vec<Value>> {
        download_from_url(&format!("--urls-file={}", file), options).await
    }

    // 处理标准输出的异步函数
    async fn process_stdout(
        mut reader: tokio::io::BufReader<tokio::process::ChildStdout>,
    ) -> Result<Vec<Value>, Box<dyn std::error::Error + Send + Sync>> {
        // 读取所有输出到缓冲区
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer).await?;

        // 使用缓冲区中的数据尝试 JSON 解析
        let stream = serde_json::Deserializer::from_slice(&buffer).into_iter::<Value>();

        // 处理JSON流并收集结果
        let mut results = Vec::new();
        let mut has_errors = false;

        for value in stream {
            match value {
                Ok(v) => {
                    // JSON解析成功，添加到结果中
                    info!("JSON解析成功");
                    results.push(v);
                }
                Err(e) => {
                    // JSON解析错误，标记有错误
                    error!("JSON解析错误: {}", e);
                    eprintln!("JSON解析错误: {}", e);
                    has_errors = true;
                }
            }
        }

        // 如果有任何解析错误，打印原始内容
        if has_errors {
            if let Ok(output) = String::from_utf8(buffer) {
                error!("JSON解析错误,即将打印原始内容");
                eprintln!("原始输出:\n{}", output);
            }
        }

        Ok(results)
    }

    // 处理标准错误的异步函数
    async fn process_stderr(
        reader: tokio::io::BufReader<tokio::process::ChildStderr>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            error!("运行出错，即将打印标准错误");
            eprintln!("STDERR: {}", line);
        }
        Ok(())
    }
}
