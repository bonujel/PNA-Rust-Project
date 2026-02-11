use serde::{Deserialize, Serialize};
use std::io::{BufReader, BufWriter, Write};
use std::net::TcpStream;

#[derive(Debug, Serialize, Deserialize)]
struct Request {
    command: String,
    key: String,
    value: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Response {
    success: bool,
    message: String,
    data: Option<String>,
}

fn main() -> std::io::Result<()> {
    // 连接到服务器
    let stream = TcpStream::connect("127.0.0.1:8080")?;
    println!("已连接到服务器: {}", stream.peer_addr()?);

    // 创建 BufReader 和 BufWriter
    let reader = BufReader::new(stream.try_clone()?);
    let mut writer = BufWriter::new(stream);

    // 发送多个请求
    let requests = vec![
        Request {
            command: "set".to_string(),
            key: "name".to_string(),
            value: Some("Alice".to_string()),
        },
        Request {
            command: "get".to_string(),
            key: "name".to_string(),
            value: None,
        },
        Request {
            command: "delete".to_string(),
            key: "name".to_string(),
            value: None,
        },
    ];

    // 创建一个用于读取响应的迭代器
    let deserializer = serde_json::Deserializer::from_reader(reader);
    let mut response_iter = deserializer.into_iter::<Response>();

    // 发送每个请求并读取响应
    for request in requests {
        println!("\n发送请求: {:?}", request);

        // 使用 serde_json::to_writer() 序列化并写入请求
        serde_json::to_writer(&mut writer, &request)?;
        writer.write_all(b"\n")?;
        writer.flush()?;

        // 读取响应
        if let Some(result) = response_iter.next() {
            match result {
                Ok(response) => {
                    println!("收到响应: {:?}", response);
                }
                Err(e) => {
                    eprintln!("解析响应失败: {}", e);
                    break;
                }
            }
        }
    }

    println!("\n客户端关闭");
    Ok(())
}
