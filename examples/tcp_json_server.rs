use serde::{Deserialize, Serialize};
use std::io::{BufReader, BufWriter, Write};
use std::net::{TcpListener, TcpStream};

/// 请求消息结构
#[derive(Debug, Serialize, Deserialize)]
struct Request {
    command: String,
    key: String,
    value: Option<String>,
}

/// 响应消息结构
#[derive(Debug, Serialize, Deserialize)]
struct Response {
    success: bool,
    message: String,
    data: Option<String>,
}

/// 处理单个客户端连接
fn handle_client(stream: TcpStream) -> std::io::Result<()> {
    println!("新客户端连接: {}", stream.peer_addr()?);

    // 为读取创建 BufReader
    let reader = BufReader::new(stream.try_clone()?);
    // 为写入创建 BufWriter
    let mut writer = BufWriter::new(stream);

    // 使用 serde_json::Deserializer 从流中读取 JSON
    // from_reader() 创建一个可以从流中反序列化多个 JSON 值的迭代器
    let deserializer = serde_json::Deserializer::from_reader(reader);
    let stream_iter = deserializer.into_iter::<Request>();

    // 处理来自客户端的每个请求
    for result in stream_iter {
        match result {
            Ok(request) => {
                println!("收到请求: {:?}", request);

                // 处理请求并生成响应
                let response = process_request(request);

                // 使用 serde_json::to_writer() 将响应写入流
                // 这会自动序列化并写入到 BufWriter
                if let Err(e) = serde_json::to_writer(&mut writer, &response) {
                    eprintln!("写入响应失败: {}", e);
                    break;
                }

                // 添加换行符以分隔多个 JSON 对象
                if let Err(e) = writer.write_all(b"\n") {
                    eprintln!("写入换行符失败: {}", e);
                    break;
                }

                // 刷新缓冲区确保数据发送
                if let Err(e) = writer.flush() {
                    eprintln!("刷新缓冲区失败: {}", e);
                    break;
                }
            }
            Err(e) => {
                eprintln!("解析请求失败: {}", e);
                break;
            }
        }
    }

    println!("客户端断开连接");
    Ok(())
}

/// 处理请求逻辑
fn process_request(request: Request) -> Response {
    match request.command.as_str() {
        "get" => Response {
            success: true,
            message: format!("获取键: {}", request.key),
            data: Some(format!("value_for_{}", request.key)),
        },
        "set" => Response {
            success: true,
            message: format!("设置键: {} = {:?}", request.key, request.value),
            data: None,
        },
        _ => Response {
            success: false,
            message: format!("未知命令: {}", request.command),
            data: None,
        },
    }
}

fn main() -> std::io::Result<()> {
    // 绑定到本地地址
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    println!("服务器监听在: {}", listener.local_addr()?);

    // 单线程同步服务器：在循环中接受连接
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                // 处理连接（阻塞直到客户端断开）
                if let Err(e) = handle_client(stream) {
                    eprintln!("处理客户端时出错: {}", e);
                }
            }
            Err(e) => {
                eprintln!("接受连接失败: {}", e);
            }
        }
    }

    Ok(())
}
