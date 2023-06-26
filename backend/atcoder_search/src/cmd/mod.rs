pub mod crawl;
pub mod generate;
pub mod post;
pub mod server;
pub mod update;

use clap::ValueEnum;
use std::fmt;

#[derive(Debug, ValueEnum, Clone)]
pub enum TargetDomain {
    Problems,
    Users,
    Recommends,
}

impl fmt::Display for TargetDomain {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TargetDomain::Problems => write!(f, "problems"),
            TargetDomain::Users => write!(f, "users"),
            TargetDomain::Recommends => write!(f, "recommends"),
        }
    }
}
