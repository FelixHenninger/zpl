#[derive(clap::Parser)]
pub struct App {
    #[clap(long, env = "ZPL_LISTEN", default_value = "0.0.0.0:3000")]
    pub listen: String,

    #[clap(long, env = "ZPL_CONFIGURATION", default_value = "server.json")]
    pub configuration: String,
}
