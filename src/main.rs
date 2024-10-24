use clap::Parser;
use home_task_smart_homes::{
    bulb::Bulb, cli::Cli, error::Error, fan::Fan, home::Home, tv::TV, DeviceStatus,
};
use paho_mqtt::{AsyncClient, QOS_0};
use tokio::{pin, select, task::JoinSet};
use tracing::{error, info, warn};
use tracing_log::AsTrace;

async fn watcher(broker_url: impl AsRef<str>) -> Result<(), Error> {
    info!("Starting watcher");
    let mut client = AsyncClient::new(broker_url.as_ref()).unwrap();
    client.connect(None).await?;

    let _ = client
        .subscribe_many_same_qos(
            &[
                "fan/home/+/status",
                "tv/home/+/status",
                "bulb/home/+/status",
            ],
            QOS_0,
        )
        .await?;

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
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    tracing_subscriber::fmt()
        .with_level(true)
        .with_max_level(cli.verbosity.log_level_filter().as_trace())
        .pretty()
        .init();

    let broker_url = cli.broker_url;

    // simulate 10 houses
    let mut join_set = JoinSet::new();
    for i in 0..cli.num_houses {
        let broker_url = broker_url.clone();
        join_set.spawn(async move {
            Home::new(
                format!("home-{i}"),
                Bulb::try_new(format!("home/{}", i), &broker_url).unwrap(),
                Fan::try_new(format!("home/{}", i), &broker_url).unwrap(),
                TV::try_new(format!("home/{}", i), &broker_url).unwrap(),
            )
            .handle_incoming()
            .await
        });
    }
    pin!(
        let join_fut = join_set.join_all();
    );

    let mut watcher_handle = tokio::spawn(watcher(broker_url));

    loop {
        select! {
            res = &mut watcher_handle => {
                let res = res?;
                if let Err(ref err) = res {
                    error!(?err, "watcher failed");
                }
                res?
            },
            results = &mut join_fut => {
                for res in results {
                    if let Err(err) = res {
                        error!(?err, "device failed");
                    }
                }
            }
        }
    }
}
