use std::io::BufWriter;
use std::io::Write;
use std::net::{TcpListener, TcpStream, ToSocketAddrs};

use log::{debug, error};
use serde_json::Deserializer;

use crate::common::{Request, Response};
use crate::engines::KvsEngine;
use crate::thread_pool::ThreadPool;
use crate::Result;

/// The server of a key-value store.
///
/// Generic over both the storage engine `E` and the thread pool `P`,
/// allowing flexible composition of concurrency strategies.
pub struct KvsServer<E: KvsEngine, P: ThreadPool> {
    engine: E,
    pool: P,
}

impl<E: KvsEngine, P: ThreadPool> KvsServer<E, P> {
    /// Creates a `KvsServer` with a given storage engine and thread pool.
    pub fn new(engine: E, pool: P) -> Self {
        Self { engine, pool }
    }

    /// Runs the server, listening for connections on the given address.
    ///
    /// Each connection is dispatched to the thread pool for handling.
    pub fn run(&self, addr: impl ToSocketAddrs) -> Result<()> {
        let listener = TcpListener::bind(addr)?;

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let engine = self.engine.clone();
                    self.pool.spawn(move || {
                        if let Err(e) = handle_connection(engine, stream) {
                            error!("Error handling connection: {}", e);
                        }
                    });
                }
                Err(e) => error!("Connection failed: {}", e),
            }
        }

        Ok(())
    }
}

/// Handles a single client connection.
fn handle_connection<E: KvsEngine>(engine: E, stream: TcpStream) -> Result<()> {
    let peer_addr = stream.peer_addr()?;
    debug!("Accepted connection from {}", peer_addr);

    let reader = &stream;
    let mut writer = BufWriter::new(&stream);
    let requests = Deserializer::from_reader(reader).into_iter::<Request>();

    for request in requests {
        let request = request?;
        debug!("Received request from {}: {:?}", peer_addr, request);

        let response = match request {
            Request::Set { key, value } => match engine.set(key, value) {
                Ok(()) => Response::Ok(None),
                Err(e) => Response::Err(e.to_string()),
            },
            Request::Get { key } => match engine.get(key) {
                Ok(value) => Response::Ok(value),
                Err(e) => Response::Err(e.to_string()),
            },
            Request::Remove { key } => match engine.remove(key) {
                Ok(()) => Response::Ok(None),
                Err(e) => Response::Err(e.to_string()),
            },
        };

        serde_json::to_writer(&mut writer, &response)?;
        writer.flush()?;
    }

    Ok(())
}
