use anyhow::Result;

use clap::Args;

#[derive(Debug, Args)]
pub struct GenerateArgs {
    #[arg(long)]
    all: bool,
}

pub async fn run(args: GenerateArgs) -> Result<()> {
    println!("generate with {:?}", args);
    Ok(())
}
