mod cmd;
mod modules;
mod types;

use crate::cmd::{
    crawl::{self, CrawlArgs},
    generate::{self, GenerateArgs},
    post::{self, PostArgs},
    server::{self, ServerArgs},
    update::{self, UpdateIndexArgs},
};
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use std::{env, str::FromStr};
use tokio::runtime::Builder;
use tracing_subscriber::{
    filter::{EnvFilter, LevelFilter},
    fmt::{self, time::OffsetTime},
};

#[derive(Debug, Parser)]
#[command(name = "atcoder_search")]
#[command(about = "AtCoder Search")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Crawl(CrawlArgs),
    Generate(GenerateArgs),
    Post(PostArgs),
    Server(ServerArgs),
    Update(UpdateIndexArgs),
}

fn main() {
    dotenv().ok();

    let log_level = env::var("RUST_LOG").unwrap_or(String::from("info"));
    let filter = EnvFilter::builder()
        .with_default_directive(
            LevelFilter::from_str(&log_level)
                .expect("couldn't parse specified log level")
                .into(),
        )
        .from_env_lossy();
    let format = fmt::format()
        .with_level(true)
        .with_target(true)
        .with_ansi(false)
        .with_thread_ids(true)
        .with_timer(OffsetTime::local_rfc_3339().unwrap());
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .event_format(format)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("failed to set tracing subscriber");

    let runtime = Builder::new_multi_thread().enable_all().build().unwrap();

    match Cli::parse().command {
        Commands::Crawl(args) => runtime.block_on(crawl::run(args)),
        Commands::Generate(args) => runtime.block_on(generate::run(args)),
        Commands::Post(args) => runtime.block_on(post::run(args)),
        Commands::Server(args) => runtime.block_on(server::run(args)),
        Commands::Update(args) => runtime.block_on(update::run(args)),
    }
    .expect("command failed");
}
