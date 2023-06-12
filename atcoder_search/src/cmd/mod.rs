pub mod crawl;
pub mod generate;
pub mod post;
pub mod server;
pub mod update;

use clap::ValueEnum;

#[derive(Debug, ValueEnum, Clone)]
pub enum TargetDomain {
    Problems,
    Users,
    Recommend,
}
