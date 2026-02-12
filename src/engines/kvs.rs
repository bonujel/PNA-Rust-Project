use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

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

/// A log-structured key-value store with lock-free readers.
///
/// Write operations are serialized via a `Mutex`. The in-memory index
/// uses `RwLock` for concurrent read access. Each clone maintains its
/// own set of file readers to avoid lock contention on reads.
pub struct KvStore {
    /// Directory where log files are stored (immutable).
    path: Arc<PathBuf>,
    /// Shared in-memory index: key -> log pointer. RwLock allows
    /// multiple concurrent readers with a single writer.
    index: Arc<RwLock<HashMap<String, CommandPos>>>,
    /// Writer-side state, protected by Mutex (single writer).
    writer: Arc<Mutex<KvStoreWriter>>,
    /// Per-clone reader handles (not shared between threads).
    reader: KvStoreReader,
}

impl Clone for KvStore {
    fn clone(&self) -> Self {
        KvStore {
            path: self.path.clone(),
            index: self.index.clone(),
            writer: self.writer.clone(),
            // Each clone gets a fresh set of readers — this is the key
            // to lock-free reads: no shared mutable reader state.
            reader: KvStoreReader {
                safe_point: self.reader.safe_point.clone(),
                path: self.path.clone(),
                readers: RefCell::new(HashMap::new()),
            },
        }
    }
}

/// Writer-side state, protected by a Mutex.
struct KvStoreWriter {
    /// Current generation number for the active log file.
    current_gen: u64,
    /// Writer for the current active log file.
    writer: BufWriterWithPos<File>,
    /// Writer's own readers (used during compaction only).
    readers: HashMap<u64, BufReaderWithPos<File>>,
    /// Number of bytes of stale (compactable) data.
    uncompacted: u64,
}

/// Per-clone reader state. Each thread gets its own instance via Clone.
struct KvStoreReader {
    /// Minimum valid generation after compaction.
    safe_point: Arc<AtomicU64>,
    /// Path to log directory (for lazy file opening).
    path: Arc<PathBuf>,
    /// Per-thread reader handles, lazily opened.
    readers: RefCell<HashMap<u64, BufReaderWithPos<File>>>,
}

impl KvStoreReader {
    /// Reads a command from the log using per-thread file handles.
    ///
    /// Lazily opens file handles as needed. Cleans up stale handles
    /// when the safe_point advances (after compaction).
    fn read_command(&self, cmd_pos: CommandPos) -> Result<Option<String>> {
        self.close_stale_readers();

        let mut readers = self.readers.borrow_mut();
        let reader = match readers.entry(cmd_pos.gen) {
            std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
            std::collections::hash_map::Entry::Vacant(e) => {
                let r = BufReaderWithPos::new(
                    File::open(log_path(&self.path, cmd_pos.gen))?,
                )?;
                e.insert(r)
            }
        };
        reader.seek(SeekFrom::Start(cmd_pos.pos))?;
        let cmd_reader = reader.take(cmd_pos.len);
        if let Command::Set { value, .. } = serde_json::from_reader(cmd_reader)? {
            Ok(Some(value))
        } else {
            Err(KvError::UnexpectedCommandType)
        }
    }

    /// Removes file handles for generations older than the safe point.
    fn close_stale_readers(&self) {
        let safe_point = self.safe_point.load(Ordering::Acquire);
        if safe_point > 0 {
            let mut readers = self.readers.borrow_mut();
            readers.retain(|&gen, _| gen >= safe_point);
        }
    }
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
            let mut reader = BufReaderWithPos::new(
                File::open(log_path(&path, gen))?,
            )?;
            uncompacted += load(gen, &mut reader, &mut index)?;
            readers.insert(gen, reader);
        }

        let current_gen = gen_list.last().unwrap_or(&0) + 1;
        let writer = new_log_file(&path, current_gen, &mut readers)?;

        let safe_point = Arc::new(AtomicU64::new(0));
        let path = Arc::new(path);

        let kv_writer = KvStoreWriter {
            current_gen,
            writer,
            readers,
            uncompacted,
        };

        let reader = KvStoreReader {
            safe_point: safe_point.clone(),
            path: path.clone(),
            readers: RefCell::new(HashMap::new()),
        };

        Ok(Self {
            path,
            index: Arc::new(RwLock::new(index)),
            writer: Arc::new(Mutex::new(kv_writer)),
            reader,
        })
    }
}

/// Compacts the log by writing only the latest values to a new log file.
///
/// After compaction, updates `safe_point` so reader threads can clean up
/// stale file handles.
fn compact(
    writer: &mut KvStoreWriter,
    index: &RwLock<HashMap<String, CommandPos>>,
    safe_point: &AtomicU64,
    path: &Path,
) -> Result<()> {
    let compaction_gen = writer.current_gen + 1;
    writer.current_gen += 2;
    writer.writer = new_log_file(path, writer.current_gen, &mut writer.readers)?;

    let mut compaction_writer =
        new_log_file(path, compaction_gen, &mut writer.readers)?;

    let mut index = index.write().unwrap();
    let mut new_pos = 0u64;
    for cmd_pos in index.values_mut() {
        let reader = writer
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
    drop(index);

    let stale_gens: Vec<u64> = writer
        .readers
        .keys()
        .filter(|&&gen| gen < compaction_gen)
        .copied()
        .collect();
    for stale_gen in stale_gens {
        writer.readers.remove(&stale_gen);
        fs::remove_file(log_path(path, stale_gen))?;
    }
    writer.uncompacted = 0;

    // Update safe_point so reader threads know to discard old handles.
    safe_point.store(compaction_gen, Ordering::Release);

    Ok(())
}

impl KvsEngine for KvStore {
    fn set(&self, key: String, value: String) -> Result<()> {
        let mut writer = self.writer.lock().unwrap();
        let cmd = Command::Set {
            key: key.clone(),
            value,
        };
        let pos = writer.writer.pos;
        serde_json::to_writer(&mut writer.writer, &cmd)?;
        writer.writer.flush()?;
        let new_pos = writer.writer.pos;
        let current_gen = writer.current_gen;

        // Write-lock the index to insert the new entry.
        let mut index = self.index.write().unwrap();
        if let Some(old_cmd) = index.insert(
            key,
            CommandPos {
                gen: current_gen,
                pos,
                len: new_pos - pos,
            },
        ) {
            writer.uncompacted += old_cmd.len;
        }
        drop(index);

        if writer.uncompacted > COMPACTION_THRESHOLD {
            compact(
                &mut writer,
                &self.index,
                &self.reader.safe_point,
                &self.path,
            )?;
        }

        Ok(())
    }

    /// Lock-free read: only acquires a RwLock read lock on the index,
    /// then uses per-thread file handles. No Mutex contention.
    #[allow(clippy::needless_pass_by_value)]
    fn get(&self, key: String) -> Result<Option<String>> {
        // Read-lock the index — multiple threads can do this concurrently.
        let index = self.index.read().unwrap();
        if let Some(cmd_pos) = index.get(&key).copied() {
            drop(index); // Release read lock as early as possible.

            // Use per-thread reader (lazy open, no shared state).
            self.reader.read_command(cmd_pos)
        } else {
            Ok(None)
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    fn remove(&self, key: String) -> Result<()> {
        // Acquire writer mutex first to serialize all writes, then check
        // existence. This prevents a TOCTOU race where another thread
        // removes the same key between our check and our write.
        let mut writer = self.writer.lock().unwrap();

        {
            let index = self.index.read().unwrap();
            if !index.contains_key(&key) {
                return Err(KvError::KeyNotFound);
            }
        }

        let cmd = Command::Remove { key: key.clone() };
        serde_json::to_writer(&mut writer.writer, &cmd)?;
        writer.writer.flush()?;

        let mut index = self.index.write().unwrap();
        if let Some(old_cmd) = index.remove(&key) {
            writer.uncompacted += old_cmd.len;
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
    let writer = BufWriterWithPos::new(
        OpenOptions::new().create(true).append(true).open(&path)?,
    )?;
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