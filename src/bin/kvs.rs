use clap::{Parser, Subcommand};
use kvs::{KvError, KvStore, Result};
use std::env::current_dir;
use std::process;

/// A simple key-value store CLI
#[derive(Parser)]
#[command(name = "kvs")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "A simple key-value store", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Set the value of a string key to a string
    Set {
        /// The key to set
        key: String,
        /// The value to set
        value: String,
    },
    /// Get the string value of a given string key
    Get {
        /// The key to get
        key: String,
    },
    /// Remove a given key
    #[command(name = "rm")]
    Remove {
        /// The key to remove
        key: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Set { key, value }) => {
            let mut store = KvStore::open(current_dir()?)?;
            store.set(key, value)?;
        }
        Some(Commands::Get { key }) => {
            let mut store = KvStore::open(current_dir()?)?;
            match store.get(key)? {
                Some(value) => print!("{value}"),
                None => print!("Key not found"),
            }
        }
        Some(Commands::Remove { key }) => {
            let mut store = KvStore::open(current_dir()?)?;
            match store.remove(key) {
                Ok(()) => {}
                Err(KvError::KeyNotFound) => {
                    print!("Key not found");
                    process::exit(1);
                }
                Err(e) => return Err(e),
            }
        }
        None => {
            process::exit(1);
        }
    }

    Ok(())
}
