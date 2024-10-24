use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};

#[derive(Debug, Parser)]
pub struct Cli {
    #[clap(short, long, default_value = "tcp://localhost:1883")]
    pub broker_url: String,

    #[clap(flatten)]
    pub verbosity: Verbosity<InfoLevel>,
}
