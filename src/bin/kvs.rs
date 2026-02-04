use clap::{Parser, Subcommand};
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

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Set { key: _, value: _ }) => {
            eprintln!("unimplemented");
            process::exit(1);
        }
        Some(Commands::Get { key: _ }) => {
            eprintln!("unimplemented");
            process::exit(1);
        }
        Some(Commands::Remove { key: _ }) => {
            eprintln!("unimplemented");
            process::exit(1);
        }
        None => {
            eprintln!("unimplemented");
            process::exit(1);
        }
    }
}
