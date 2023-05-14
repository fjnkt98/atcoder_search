use anyhow::Result;
use clap::Args;

#[derive(Debug, Args)]
pub struct CrawlArgs {
    #[arg(long)]
    all: bool,
}

pub async fn run(args: CrawlArgs) -> Result<()> {
    println!("crawl with {:?}", args);
    Ok(())
}
