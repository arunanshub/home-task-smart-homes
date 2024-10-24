use home_task_smart_homes::{bulb::Bulb, fan::Fan, tv::TV, House};
use tokio::task::JoinSet;
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
    let broker_url = "tcp://localhost:1883";

    let mut join_set = JoinSet::new();
    for i in 0..10 {
        join_set.spawn(async move {
            House::new(
                format!("home-{i}"),
                Bulb::try_new(i.to_string(), broker_url).unwrap(),
                Fan::try_new(i.to_string(), broker_url).unwrap(),
                TV::try_new(i.to_string(), broker_url).unwrap(),
            )
            .handle_incoming()
            .await
            .unwrap();
        });
    }

    join_set.join_all().await;
    Ok(())
}
