use std::sync::Arc;
use std::sync::mpsc::channel;
use clap::{Parser, Subcommand};


use log::{debug, error, info};
use crate::config::create;
use crate::discordbot::create_client;
use crate::replacer::run_replacer_tests;

mod replacer;
mod discordbot;
mod config;
mod resource;


#[derive(Debug, Parser)]
#[command(name = "linksbot")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Test {
        /// Replacer name, leave empty to run tests on all replacers
        replacer_name: Option<String>,
    },
    Run {},
    Version {},
}

async fn start() {
    let mut threads = Vec::new();

    let mut config = Arc::new(config::create().unwrap());

    let mut client = create_client(config.clone()).await.unwrap();

    let shard_manager = client.shard_manager.clone();

    let (tx, rx) = channel();

    ctrlc::set_handler(move || {
        debug!("got ctrl+c");
        tx.send(()).expect("failed to send signal on channel");
        debug!("sent signal on channel")
    }).expect("error setting CTRL-C handler");

    threads.push(tokio::spawn(async move {
        debug!("starting client...");

        if let Err(err) = client.start().await {
            error!("client fatal error: {}", err);
        }

        debug!("client stopped running");
    }));


    info!("press CTRL+C to exit");
    rx.recv().expect("failed to receive signal");
    info!("CTRL+C received, shutting down...");
    shard_manager.shutdown_all().await;

    info!("shut down...");

    threads.iter().for_each(|thread| thread.abort());

    std::process::exit(0);
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "discord_links_bot=info")
    }
    env_logger::init();


    let args = Cli::parse();

    match args.command {
        Commands::Test { replacer_name } => {
            let config = Arc::new(create().expect("config file could not be read"));
            match run_replacer_tests(replacer_name, config).await {
                Ok(_) => {
                    info!("successfully ran tests")
                }
                Err(err) => {
                    error!("failed to run tests: {}", err)
                }
            };
        }
        Commands::Run {} => {
            start().await;
        }
        Commands::Version {} => {
            info!("{} {} @ {}", option_env!("GIT_TAG").unwrap_or(""), env!("GIT_HASH"), env!("COMMIT_DATE"));
        }
    }

    Ok(())
}

