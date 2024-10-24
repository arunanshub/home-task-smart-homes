use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};

#[derive(Debug, Parser)]
pub struct Cli {
    #[clap(short, long, default_value = "tcp://localhost:1883")]
    pub broker_url: String,

    /// Number of houses to simulate.
    #[clap(short, long, default_value_t = 10, value_parser = validate_num_houses)]
    pub num_houses: u32,

    #[clap(flatten)]
    pub verbosity: Verbosity<InfoLevel>,
}

// not using value_parser!(u32).range(1..) because the error message is weird.
fn validate_num_houses(v: &str) -> Result<u32, String> {
    match v.parse::<u32>() {
        Ok(v) => {
            if v == 0 {
                Err("num_houses must be greater than 0".to_string())
            } else {
                Ok(v)
            }
        }
        Err(_) => Err("num_houses must be a number".to_string()),
    }
}
