use anyhow::Result;
use clap::Args;

#[derive(Debug, Args)]
pub struct ServerArgs {
    #[arg(long)]
    all: bool,
}

pub async fn run(args: ServerArgs) -> Result<()> {
    println!("start server with {:?}", args);
    Ok(())
}
