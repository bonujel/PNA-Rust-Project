use std::net::SocketAddr;
use std::process::exit;

use clap::{Parser, Subcommand};

use kvs::KvsClient;

const DEFAULT_ADDR: &str = "127.0.0.1:4000";

#[derive(Parser)]
#[command(name = "kvs-client", version, about = "A key-value store client")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Set the value of a string key to a string
    Set {
        /// The key
        key: String,
        /// The value
        value: String,
        /// Server address
        #[arg(long, default_value = DEFAULT_ADDR, value_name = "IP-PORT")]
        addr: SocketAddr,
    },
    /// Get the string value of a given string key
    Get {
        /// The key
        key: String,
        /// Server address
        #[arg(long, default_value = DEFAULT_ADDR, value_name = "IP-PORT")]
        addr: SocketAddr,
    },
    /// Remove a given key
    Rm {
        /// The key
        key: String,
        /// Server address
        #[arg(long, default_value = DEFAULT_ADDR, value_name = "IP-PORT")]
        addr: SocketAddr,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Set { key, value, addr } => {
            let mut client = KvsClient::connect(addr).unwrap_or_else(|e| {
                eprintln!("Failed to connect to server: {}", e);
                exit(1);
            });
            if let Err(e) = client.set(key, value) {
                eprintln!("{}", e);
                exit(1);
            }
        }
        Commands::Get { key, addr } => {
            let mut client = KvsClient::connect(addr).unwrap_or_else(|e| {
                eprintln!("Failed to connect to server: {}", e);
                exit(1);
            });
            match client.get(key) {
                Ok(Some(value)) => println!("{}", value),
                Ok(None) => println!("Key not found"),
                Err(e) => {
                    eprintln!("{}", e);
                    exit(1);
                }
            }
        }
        Commands::Rm { key, addr } => {
            let mut client = KvsClient::connect(addr).unwrap_or_else(|e| {
                eprintln!("Failed to connect to server: {}", e);
                exit(1);
            });
            if let Err(e) = client.remove(key) {
                eprintln!("{}", e);
                exit(1);
            }
        }
    }
}
