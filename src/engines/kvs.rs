use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Deserializer;

use super::KvsEngine;
use crate::{KvError, Result};

/// Compaction threshold in bytes.
const COMPACTION_THRESHOLD: u64 = 1024 * 1024;

/// Represents a command that can be serialized to the log.
#[derive(Serialize, Deserialize, Debug)]
enum Command {
    Set { key: String, value: String },
    Remove { key: String },
}

/// Pointer to a command's position in the log.
#[derive(Debug, Clone, Copy)]
struct CommandPos {
    /// Log file generation number.
    gen: u64,
    /// Byte offset of the command in the file.
    pos: u64,
    /// Length of the serialized command in bytes.
    len: u64,
}

/// A log-structured key-value store.
///
/// Data is persisted to disk using append-only log files.
/// An in-memory index maps keys to their positions in the log.
pub struct KvStore {
    /// Directory where log files are stored.
    path: PathBuf,
    /// Current generation number for the active log file.
    current_gen: u64,
    /// Writer for the current active log file.
    writer: BufWriterWithPos<File>,
    /// Readers for each log file generation.
    readers: HashMap<u64, BufReaderWithPos<File>>,
    /// In-memory index: key -> log pointer.
    index: HashMap<String, CommandPos>,
    /// Number of bytes of stale (compactable) data.
    uncompacted: u64,
}

impl KvStore {
    /// Opens a `KvStore` at the given path.
    ///
    /// Creates the directory if it does not exist.
    /// Replays existing log files to rebuild the in-memory index.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        fs::create_dir_all(&path)?;

        let mut readers = HashMap::new();
        let mut index = HashMap::new();
        let mut uncompacted = 0u64;

        let gen_list = sorted_gen_list(&path)?;
        for &gen in &gen_list {
            let mut reader = BufReaderWithPos::new(File::open(log_path(&path, gen))?)?;
            uncompacted += load(gen, &mut reader, &mut index)?;
            readers.insert(gen, reader);
        }

        let current_gen = gen_list.last().unwrap_or(&0) + 1;
        let writer = new_log_file(&path, current_gen, &mut readers)?;

        Ok(Self {
            path,
            current_gen,
            writer,
            readers,
            index,
            uncompacted,
        })
    }

    /// Compacts the log by writing only the latest values to a new log file.
    fn compact(&mut self) -> Result<()> {
        let compaction_gen = self.current_gen + 1;
        self.current_gen += 2;
        self.writer = new_log_file(&self.path, self.current_gen, &mut self.readers)?;

        let mut compaction_writer = new_log_file(&self.path, compaction_gen, &mut self.readers)?;

        let mut new_pos = 0u64;
        for cmd_pos in self.index.values_mut() {
            let reader = self
                .readers
                .get_mut(&cmd_pos.gen)
                .ok_or(KvError::LogFileNotFound(cmd_pos.gen))?;
            reader.seek(SeekFrom::Start(cmd_pos.pos))?;

            let mut entry_reader = reader.take(cmd_pos.len);
            let len = io::copy(&mut entry_reader, &mut compaction_writer)?;
            *cmd_pos = CommandPos {
                gen: compaction_gen,
                pos: new_pos,
                len,
            };
            new_pos += len;
        }
        compaction_writer.flush()?;

        let stale_gens: Vec<u64> = self
            .readers
            .keys()
            .filter(|&&gen| gen < compaction_gen)
            .copied()
            .collect();
        for stale_gen in stale_gens {
            self.readers.remove(&stale_gen);
            fs::remove_file(log_path(&self.path, stale_gen))?;
        }
        self.uncompacted = 0;

        Ok(())
    }
}

impl KvsEngine for KvStore {
    fn set(&mut self, key: String, value: String) -> Result<()> {
        let cmd = Command::Set {
            key: key.clone(),
            value,
        };
        let pos = self.writer.pos;
        serde_json::to_writer(&mut self.writer, &cmd)?;
        self.writer.flush()?;
        let new_pos = self.writer.pos;

        if let Some(old_cmd) = self.index.insert(
            key,
            CommandPos {
                gen: self.current_gen,
                pos,
                len: new_pos - pos,
            },
        ) {
            self.uncompacted += old_cmd.len;
        }

        if self.uncompacted > COMPACTION_THRESHOLD {
            self.compact()?;
        }

        Ok(())
    }

    #[allow(clippy::needless_pass_by_value)]
    fn get(&mut self, key: String) -> Result<Option<String>> {
        if let Some(cmd_pos) = self.index.get(&key) {
            let reader = self
                .readers
                .get_mut(&cmd_pos.gen)
                .ok_or(KvError::LogFileNotFound(cmd_pos.gen))?;
            reader.seek(SeekFrom::Start(cmd_pos.pos))?;
            let cmd_reader = reader.take(cmd_pos.len);
            if let Command::Set { value, .. } = serde_json::from_reader(cmd_reader)? {
                Ok(Some(value))
            } else {
                Err(KvError::UnexpectedCommandType)
            }
        } else {
            Ok(None)
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    fn remove(&mut self, key: String) -> Result<()> {
        if !self.index.contains_key(&key) {
            return Err(KvError::KeyNotFound);
        }

        let cmd = Command::Remove { key: key.clone() };
        serde_json::to_writer(&mut self.writer, &cmd)?;
        self.writer.flush()?;

        if let Some(old_cmd) = self.index.remove(&key) {
            self.uncompacted += old_cmd.len;
        }

        Ok(())
    }
}

/// Returns sorted list of generation numbers from log files in the directory.
fn sorted_gen_list(path: &Path) -> Result<Vec<u64>> {
    let mut gen_list: Vec<u64> = fs::read_dir(path)?
        .flat_map(|res| -> Result<_> { Ok(res?.path()) })
        .filter(|path| path.is_file() && path.extension() == Some("log".as_ref()))
        .filter_map(|path| {
            path.file_name()
                .and_then(OsStr::to_str)
                .map(|s| s.trim_end_matches(".log"))
                .map(str::parse::<u64>)
        })
        .flatten()
        .collect();
    gen_list.sort_unstable();
    Ok(gen_list)
}

/// Loads a single log file and populates the index.
fn load(
    gen: u64,
    reader: &mut BufReaderWithPos<File>,
    index: &mut HashMap<String, CommandPos>,
) -> Result<u64> {
    let mut pos = reader.seek(SeekFrom::Start(0))?;
    let mut stream = Deserializer::from_reader(reader).into_iter::<Command>();
    let mut uncompacted = 0u64;

    while let Some(cmd) = stream.next() {
        let new_pos = stream.byte_offset() as u64;
        match cmd? {
            Command::Set { key, .. } => {
                if let Some(old_cmd) = index.insert(
                    key,
                    CommandPos {
                        gen,
                        pos,
                        len: new_pos - pos,
                    },
                ) {
                    uncompacted += old_cmd.len;
                }
            }
            Command::Remove { key } => {
                if let Some(old_cmd) = index.remove(&key) {
                    uncompacted += old_cmd.len;
                }
                uncompacted += new_pos - pos;
            }
        }
        pos = new_pos;
    }

    Ok(uncompacted)
}

/// Creates a new log file and registers its reader.
fn new_log_file(
    path: &Path,
    gen: u64,
    readers: &mut HashMap<u64, BufReaderWithPos<File>>,
) -> Result<BufWriterWithPos<File>> {
    let path = log_path(path, gen);
    let writer = BufWriterWithPos::new(OpenOptions::new().create(true).append(true).open(&path)?)?;
    readers.insert(gen, BufReaderWithPos::new(File::open(&path)?)?);
    Ok(writer)
}

/// Returns the path for a log file with the given generation number.
fn log_path(dir: &Path, gen: u64) -> PathBuf {
    dir.join(format!("{gen}.log"))
}

/// A `BufReader` that tracks the current read position.
struct BufReaderWithPos<R: Read + Seek> {
    reader: BufReader<R>,
    pos: u64,
}

impl<R: Read + Seek> BufReaderWithPos<R> {
    fn new(mut inner: R) -> Result<Self> {
        let pos = inner.stream_position()?;
        Ok(Self {
            reader: BufReader::new(inner),
            pos,
        })
    }
}

impl<R: Read + Seek> Read for BufReaderWithPos<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = self.reader.read(buf)?;
        self.pos += len as u64;
        Ok(len)
    }
}

impl<R: Read + Seek> Seek for BufReaderWithPos<R> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.pos = self.reader.seek(pos)?;
        Ok(self.pos)
    }
}

/// A `BufWriter` that tracks the current write position.
struct BufWriterWithPos<W: Write + Seek> {
    writer: BufWriter<W>,
    pos: u64,
}

impl<W: Write + Seek> BufWriterWithPos<W> {
    fn new(mut inner: W) -> Result<Self> {
        let pos = inner.seek(SeekFrom::End(0))?;
        Ok(Self {
            writer: BufWriter::new(inner),
            pos,
        })
    }
}

impl<W: Write + Seek> Write for BufWriterWithPos<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = self.writer.write(buf)?;
        self.pos += len as u64;
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl<W: Write + Seek> Seek for BufWriterWithPos<W> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.pos = self.writer.seek(pos)?;
        Ok(self.pos)
    }
}
