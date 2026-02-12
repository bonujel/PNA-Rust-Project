use std::env::current_dir;
use std::fs;
use std::net::SocketAddr;
use std::process::exit;

use clap::Parser;
use log::{error, info};

use kvs::{
    KvError, KvStore, KvsEngine, KvsServer, Result, SharedQueueThreadPool, SledKvsEngine,
    ThreadPool,
};

const DEFAULT_ADDR: &str = "127.0.0.1:4000";
const DEFAULT_ENGINE: &str = "kvs";

#[derive(Parser)]
#[command(name = "kvs-server", version, about = "A key-value store server")]
struct Cli {
    /// Server listening address
    #[arg(long, default_value = DEFAULT_ADDR, value_name = "IP-PORT")]
    addr: SocketAddr,

    /// Storage engine: "kvs" or "sled"
    #[arg(long, value_name = "ENGINE-NAME")]
    engine: Option<String>,
}

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .target(env_logger::Target::Stderr)
        .init();

    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        error!("{}", e);
        exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    let engine_name = resolve_engine(cli.engine)?;
    let num_cpus = num_cpus::get() as u32;

    info!("kvs-server {}", env!("CARGO_PKG_VERSION"));
    info!("Storage engine: {}", engine_name);
    info!("Listening on {}", cli.addr);

    match engine_name.as_str() {
        "kvs" => run_with_engine(
            KvStore::open(current_dir()?)?,
            SharedQueueThreadPool::new(num_cpus)?,
            cli.addr,
        ),
        "sled" => run_with_engine(
            SledKvsEngine::new(sled::open(current_dir()?)?),
            SharedQueueThreadPool::new(num_cpus)?,
            cli.addr,
        ),
        _ => unreachable!(),
    }
}

fn run_with_engine<E: KvsEngine, P: ThreadPool>(
    engine: E,
    pool: P,
    addr: SocketAddr,
) -> Result<()> {
    let server = KvsServer::new(engine, pool);
    server.run(addr)
}

/// Resolves the engine name, checking for conflicts with previously used engine.
fn resolve_engine(engine: Option<String>) -> Result<String> {
    let engine_file = current_dir()?.join("engine");
    let prev_engine = fs::read_to_string(&engine_file).ok();

    let engine = match (engine, prev_engine) {
        (Some(e), None) => e,
        (Some(e), Some(prev)) => {
            if e != prev {
                return Err(KvError::StringError(format!(
                    "Wrong engine! Previously used '{}', but '{}' was requested.",
                    prev, e
                )));
            }
            e
        }
        (None, Some(prev)) => prev,
        (None, None) => DEFAULT_ENGINE.to_owned(),
    };

    if engine != "kvs" && engine != "sled" {
        return Err(KvError::StringError(format!(
            "Invalid engine: {}. Must be 'kvs' or 'sled'.",
            engine
        )));
    }

    fs::write(current_dir()?.join("engine"), &engine)?;

    Ok(engine)
}
