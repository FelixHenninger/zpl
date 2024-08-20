use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    zpl::run(zpl::Args::parse()).await
}
