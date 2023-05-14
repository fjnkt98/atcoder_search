use anyhow::Result;
use clap::Args;

#[derive(Debug, Args)]
pub struct PostArgs {
    #[arg(long)]
    all: bool,
}

pub async fn run(args: PostArgs) -> Result<()> {
    println!("post with {:?}", args);
    Ok(())
}
