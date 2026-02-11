use std::io::{BufWriter, Write};
use std::net::{TcpStream, ToSocketAddrs};

use serde::Deserialize;
use serde_json::de::IoRead;
use serde_json::Deserializer;

use crate::common::{Request, Response};
use crate::{KvError, Result};

/// The client of a key-value store.
pub struct KvsClient {
    reader: Deserializer<IoRead<TcpStream>>,
    writer: BufWriter<TcpStream>,
}

impl KvsClient {
    /// Connects to the server at the given address.
    pub fn connect(addr: impl ToSocketAddrs) -> Result<Self> {
        let reader_stream = TcpStream::connect(&addr)?;
        let writer_stream = reader_stream.try_clone()?;
        Ok(Self {
            reader: Deserializer::from_reader(reader_stream),
            writer: BufWriter::new(writer_stream),
        })
    }

    /// Sets a key-value pair on the server.
    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        let request = Request::Set { key, value };
        serde_json::to_writer(&mut self.writer, &request)?;
        self.writer.flush()?;

        match Response::deserialize(&mut self.reader)? {
            Response::Ok(_) => Ok(()),
            Response::Err(msg) => Err(KvError::StringError(msg)),
        }
    }

    /// Gets the value for a key from the server.
    pub fn get(&mut self, key: String) -> Result<Option<String>> {
        let request = Request::Get { key };
        serde_json::to_writer(&mut self.writer, &request)?;
        self.writer.flush()?;

        match Response::deserialize(&mut self.reader)? {
            Response::Ok(value) => Ok(value),
            Response::Err(msg) => Err(KvError::StringError(msg)),
        }
    }

    /// Removes a key from the server.
    pub fn remove(&mut self, key: String) -> Result<()> {
        let request = Request::Remove { key };
        serde_json::to_writer(&mut self.writer, &request)?;
        self.writer.flush()?;

        match Response::deserialize(&mut self.reader)? {
            Response::Ok(_) => Ok(()),
            Response::Err(msg) => Err(KvError::StringError(msg)),
        }
    }
}
