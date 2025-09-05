use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use tracing::error;
use tracing::info;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Cursor;
use std::io::Write;
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

use super::config::AppConfig;
use crate::utils::httpserver::config::ImageData;
use crate::utils::httpserver::multipart::MultipartParser;
use anyhow::{Context, Result, anyhow};

/// HTTP 服务器
pub struct HttpServer {
    server: tiny_http::Server,
    config: Arc<RwLock<AppConfig>>,
    should_stop: Arc<AtomicBool>,
    // 修改：使用RwLock而不是Mutex
    images: Arc<RwLock<HashMap<String, ImageData>>>,
}

impl HttpServer {
    /// 创建新的 HTTP 服务器
    pub fn new(
        config: Arc<RwLock<AppConfig>>,
        should_stop: Arc<AtomicBool>,
        images: Arc<RwLock<HashMap<String, ImageData>>>,
    ) -> Result<Self> {
        let port = config.read().unwrap().server_port; // 使用read()而不是lock()
        let server =
            Server::http(("0.0.0.0", port)).map_err(|_| anyhow!("Failed to create HTTP server"))?;

        Ok(Self {
            server,
            config,
            should_stop,
            images,
        })
    }

    /// 运行服务器
    pub fn run(&self) -> Result<()> {
        let port = self.config.read().unwrap().server_port;
        info!("HTTP server started on port {}", port);

        // 使用非阻塞模式，以便定期检查停止标志
        self.server.unblock();

        // 使用 recv_timeout 而不是 incoming_requests
        while !self.should_stop.load(Ordering::Relaxed) {
            match self.server.recv_timeout(Duration::from_millis(100)) {
                Ok(Some(mut request)) => {
                    // 处理请求
                    let response = self.handle_request(&mut request);

                    // 发送响应
                    if let Err(e) = request.respond(response) {
                        error!("Failed to send response: {}", e);
                    }
                }
                Ok(None) => {
                    // 超时，继续检查停止标志
                    continue;
                }
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::WouldBlock {
                        // 非阻塞模式下正常，继续检查停止标志
                        continue;
                    } else {
                        error!("Error receiving request: {}", e);
                        break;
                    }
                }
            }
        }

        info!("HTTP server stopped");
        Ok(())
    }

    /// 停止服务器
    pub fn stop(&self) -> Result<()> {
        // 设置停止标志
        self.should_stop.store(true, Ordering::Relaxed);

        // 发送一个请求来中断服务器的阻塞等待
        let port = self.config.read().unwrap().server_port;
        if let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port)) {
            stream.set_write_timeout(Some(Duration::from_millis(100)))?;
            let _ = stream.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n");
        }

        Ok(())
    }

    /// 处理停止请求
    fn handle_stop(&self, headers: Vec<Header>) -> Response<Cursor<Vec<u8>>> {
        info!("Received stop request");

        // 在实际应用中，您可能需要更复杂的停止机制
        // 这里只是返回一个响应，不会实际停止服务器
        let mut response =
            Response::from_string("Stop endpoint reached").with_status_code(StatusCode(200));
        for header in headers {
            response.add_header(header);
        }
        response
    }

    /// 处理 OPTIONS 请求
    fn handle_options(&self, headers: Vec<Header>) -> Response<Cursor<Vec<u8>>> {
        let mut response = Response::from_data(Vec::new()).with_status_code(StatusCode(200));
        for header in headers {
            response.add_header(header);
        }
        response
    }

    /// 处理配置获取请求
    fn handle_get_config(&self, headers: Vec<Header>) -> Response<Cursor<Vec<u8>>> {
        let config = self.config.read().unwrap(); // 使用read()而不是lock()
        let response_body = serde_json::json!({
            "regexPattern": config.regex_pattern,
            "outputPath": config.output_path.to_string_lossy(),
            "waitTime": config.wait_time,
            "sendToRust": config.send_to_rust,
            "serverPort": config.server_port,
            "openDownload": config.open_download
        })
        .to_string();

        let mut response = Response::from_string(response_body).with_status_code(StatusCode(200));
        for header in headers {
            response.add_header(header);
        }
        response
    }

    /// 处理文件上传请求
    fn handle_upload(
        &self,
        request: &mut Request,
        headers: Vec<Header>,
    ) -> Response<Cursor<Vec<u8>>> {
        // 先获取内容类型
        let content_type = {
            request
                .headers()
                .iter()
                .find(|h| h.field.as_str().to_ascii_lowercase() == "content-type")
                .map(|h| h.value.as_str())
                .unwrap_or("")
                .to_string()
        };

        // 处理上传
        let result = self.process_upload(request.as_reader(), &content_type);

        let (status_code, message) = match result {
            Ok(filename) => (StatusCode(200), format!("Image processed: {}", filename)),
            Err(e) => {
                error!("Upload error: {}", e);
                (StatusCode(500), format!("Error: {}", e))
            }
        };

        let mut response = Response::from_string(message).with_status_code(status_code);
        for header in headers {
            response.add_header(header);
        }
        response
    }

    /// 处理未找到的请求
    fn handle_not_found(&self, headers: Vec<Header>) -> Response<Cursor<Vec<u8>>> {
        let mut response = Response::from_string("Not found").with_status_code(StatusCode(404));
        for header in headers {
            response.add_header(header);
        }
        response
    }

    /// 设置 CORS 头
    fn cors_headers(&self) -> Vec<Header> {
        vec![
            Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap(),
            Header::from_bytes("Access-Control-Allow-Methods", "GET, POST, OPTIONS").unwrap(),
            Header::from_bytes("Access-Control-Allow-Headers", "Content-Type").unwrap(),
        ]
    }

    /// 新增：处理通过URL获取图片数据的请求
    fn handle_get_image(
        &self,
        request: &Request,
        headers: Vec<Header>,
    ) -> Response<Cursor<Vec<u8>>> {
        // 从查询参数中获取URL
        let url = request.url().split('?').nth(1).and_then(|query| {
            query
                .split('&')
                .find(|param| param.starts_with("url="))
                .map(|param| param[4..].to_string()) // "url="的长度是4
        });

        let response_body = match url {
            Some(url) => {
                let images = self.images.read().unwrap();
                match images.get(&url) {
                    Some(image_data) => {
                        serde_json::to_string(&image_data).unwrap_or_else(|_| "{}".to_string())
                    }
                    None => {
                        format!("{{\"error\": \"No image found for URL: {}\"}}", url)
                    }
                }
            }
            None => "{\"error\": \"URL parameter is required\"}".to_string(),
        };

        let mut response = Response::from_string(response_body).with_status_code(StatusCode(200));
        for header in headers {
            response.add_header(header);
        }
        response
    }

    /// 修改处理HTTP请求的方法
    fn handle_request(&self, request: &mut Request) -> Response<Cursor<Vec<u8>>> {
        // 设置 CORS 头
        let cors_headers = self.cors_headers();

        match (request.method(), request.url()) {
            (Method::Options, _) => self.handle_options(cors_headers),
            (Method::Get, "/config") => self.handle_get_config(cors_headers),
            (Method::Post, "/upload") => self.handle_upload(request, cors_headers),
            (Method::Get, "/stop") => self.handle_stop(cors_headers),
            (Method::Get, "/images") => self.handle_get_images(cors_headers),
            (Method::Get, path) if path.starts_with("/image?") => {
                self.handle_get_image(request, cors_headers)
            }
            _ => self.handle_not_found(cors_headers),
        }
    }

    /// 修改：处理获取所有图片数据的请求
    fn handle_get_images(&self, headers: Vec<Header>) -> Response<Cursor<Vec<u8>>> {
        let images = self.images.read().unwrap(); // 使用read()而不是lock()
        let response_body = serde_json::to_string(&*images).unwrap_or_else(|_| "{}".to_string());

        let mut response = Response::from_string(response_body).with_status_code(StatusCode(200));
        for header in headers {
            response.add_header(header);
        }
        response
    }

    /// 修改：处理上传并转换为Base64
    fn process_upload(&self, reader: &mut dyn std::io::Read, content_type: &str) -> Result<String> {
        // 解析multipart数据
        let parsed_data = MultipartParser::parse_multipart(reader, content_type)
            .context("Failed to parse multipart data")?;

        // 获取文件数据和相关信息
        let file_data = parsed_data
            .file_data
            .ok_or_else(|| anyhow!("No file data found"))?;
        let filename = parsed_data
            .filename
            .ok_or_else(|| anyhow!("No filename found"))?;
        let url = parsed_data.url.ok_or_else(|| anyhow!("No URL found"))?;
        let mime_type = parsed_data
            .mime_type
            .ok_or_else(|| anyhow!("No MIME type found"))?;

        // 将文件数据转换为Base64
        let base64_data = BASE64.encode(&file_data);

        // 处理文件保存
        let file_path = if self.config.read().unwrap().save_to_file {
            // 确保输出目录存在
            let output_path = &self.config.read().unwrap().output_path;
            if !output_path.exists() {
                fs::create_dir_all(output_path).context("Failed to create output directory")?;
            }

            // 生成文件路径
            let file_path = output_path.join(&filename);

            // 保存文件
            let mut file = File::create(&file_path).context("Failed to create file")?;
            file.write_all(&file_data)
                .context("Failed to write file data")?;

            // 返回文件路径字符串
            Some(file_path.to_string_lossy().to_string())
        } else {
            None
        };

        // 创建ImageData
        let image_data = ImageData {
            u8_data: file_data,
            base64_data,
            filename: filename.clone(),
            mime_type,
            file_path
        };
        // println!("已保存 url:{}\tfilename:{}",url, image_data.filename);
        // 添加到图片HashMap，使用URL作为键
        self.images.write().unwrap().insert(url.clone(), image_data); // 使用write()而不是lock()

        Ok(filename)
    }
}
