use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    zpl::run(zpl::Args::parse()).await
}
