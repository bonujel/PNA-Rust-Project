use std::io::BufWriter;
use std::io::Write;
use std::net::{TcpListener, TcpStream, ToSocketAddrs};

use log::{debug, error};
use serde_json::Deserializer;

use crate::common::{Request, Response};
use crate::engines::KvsEngine;
use crate::Result;

/// The server of a key-value store.
pub struct KvsServer<E: KvsEngine> {
    engine: E,
}

impl<E: KvsEngine> KvsServer<E> {
    /// Creates a `KvsServer` with a given storage engine.
    pub fn new(engine: E) -> Self {
        Self { engine }
    }

    /// Runs the server, listening for connections on the given address.
    ///
    /// Each connection is handled synchronously in a single thread.
    pub fn run(&mut self, addr: impl ToSocketAddrs) -> Result<()> {
        let listener = TcpListener::bind(addr)?;

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    if let Err(e) = self.handle_connection(stream) {
                        error!("Error handling connection: {}", e);
                    }
                }
                Err(e) => error!("Connection failed: {}", e),
            }
        }

        Ok(())
    }

    /// Handles a single client connection.
    fn handle_connection(&mut self, stream: TcpStream) -> Result<()> {
        let peer_addr = stream.peer_addr()?;
        debug!("Accepted connection from {}", peer_addr);

        let reader = &stream;
        let mut writer = BufWriter::new(&stream);
        let requests = Deserializer::from_reader(reader).into_iter::<Request>();

        for request in requests {
            let request = request?;
            debug!("Received request from {}: {:?}", peer_addr, request);

            let response = match request {
                Request::Set { key, value } => match self.engine.set(key, value) {
                    Ok(()) => Response::Ok(None),
                    Err(e) => Response::Err(e.to_string()),
                },
                Request::Get { key } => match self.engine.get(key) {
                    Ok(value) => Response::Ok(value),
                    Err(e) => Response::Err(e.to_string()),
                },
                Request::Remove { key } => match self.engine.remove(key) {
                    Ok(()) => Response::Ok(None),
                    Err(e) => Response::Err(e.to_string()),
                },
            };

            serde_json::to_writer(&mut writer, &response)?;
            writer.flush()?;
        }

        Ok(())
    }
}
