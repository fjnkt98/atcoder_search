use anyhow::Result;
use clap::Args;

#[derive(Debug, Args)]
pub struct UpdateIndexArgs {
    #[arg(long)]
    all: bool,
}

pub async fn run(args: UpdateIndexArgs) -> Result<()> {
    println!("update index with {:?}", args);
    Ok(())
}
