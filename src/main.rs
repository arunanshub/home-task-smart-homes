use home_task_smart_homes::{bulb::Bulb, fan::Fan, home::Home, tv::TV, DeviceStatus};
use paho_mqtt::{AsyncClient, QOS_0};
use tokio::{pin, select, task::JoinSet};
use tracing::{info, level_filters::LevelFilter, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_level(true)
        .with_max_level(LevelFilter::INFO)
        .pretty()
        .init();

    let broker_url = "tcp://localhost:1883";

    // simulate 10 houses
    let mut join_set = JoinSet::new();
    for i in 0..10 {
        join_set.spawn(async move {
            Home::new(
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
    pin!(
        let join_fut = join_set.join_all();
    );

    let mut watcher = tokio::spawn(async move {
        info!("Starting watcher");
        let mut client = AsyncClient::new(broker_url).unwrap();
        client.connect(None).await.unwrap();

        let _ = client
            .subscribe_many_same_qos(&["fan/+/status", "tv/+/status", "bulb/+/status"], QOS_0)
            .await
            .unwrap();

        let stream = client.get_stream(16);
        while let Ok(Some(msg)) = stream.recv().await {
            let Ok(status) = serde_json::from_slice(msg.payload()) else {
                warn!("Failed to parse message: {:?}", msg);
                continue;
            };

            match status {
                DeviceStatus::Bulb(status) => {
                    info!(?status, "bulb status");
                }
                DeviceStatus::Fan(status) => {
                    info!(?status, "fan status");
                }
                DeviceStatus::TV(status) => {
                    info!(?status, "TV status");
                }
            }
        }
    });

    loop {
        select! {
            res = &mut watcher => {res?},
            _ = &mut join_fut => {}
        }
    }
}
