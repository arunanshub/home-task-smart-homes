use home_task_smart_homes::{bulb::Bulb, fan::Fan};
use tokio::select;
use tracing::level_filters::LevelFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_level(true)
        .with_max_level(LevelFilter::INFO)
        .pretty()
        .init();

    // let client_opts = CreateOptions::new();
    // dbg!(client_opts);
    let mut bulb = Bulb::try_new("1", "tcp://localhost:1883").unwrap();
    let mut fan = Fan::try_new("1", "tcp://localhost:1883").unwrap();
    let mut fan2 = Fan::try_new("2", "tcp://localhost:1883").unwrap();

    let mut fan2_handle = tokio::spawn(async move { fan2.handle_incoming().await });
    let mut fan_handle = tokio::spawn(async move { fan.handle_incoming().await });
    let mut bulb_handle = tokio::spawn(async move { bulb.handle_incoming().await });

    loop {
        select! {
            res = &mut fan_handle => { res?? }
            res = &mut fan2_handle => { res?? }
            res = &mut bulb_handle => { res?? }
        }
    }
}
