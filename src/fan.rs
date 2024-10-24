use crate::{error::Error, DeviceStatus};
use chrono::{DateTime, Utc};
use educe::Educe;
use paho_mqtt::{AsyncClient, ConnectOptionsBuilder, CreateOptionsBuilder, Message, QOS_1};
use parking_lot::Mutex;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{sync::Arc, time::Duration};
use tokio::{select, task::JoinHandle, time::sleep};
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FanState {
    pub is_on: bool,
    pub speed: u8,
    pub voltage: f32,
}

#[derive(Educe, Clone)]
#[educe(Debug)]
pub struct Fan {
    #[educe(Debug(ignore))]
    client: AsyncClient,
    pub id: String,
    state: Arc<Mutex<FanState>>,
}

#[serde_with::serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanStatus {
    pub id: String,
    pub is_on: bool,
    pub speed: u8,
    pub voltage: f32,
    #[serde_as(as = "serde_with::TimestampSeconds<i64, serde_with::formats::Flexible>")]
    pub timestamp: DateTime<Utc>,
}

/// Commands that can be recieved by the fan.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "cmd", content = "args", rename_all = "snake_case")]
pub enum FanCommand {
    /// Turn on the fan.
    On,
    /// Turn off the fan.
    Off,
    /// Set speed.
    Speed(u8),
}

impl Fan {
    pub fn try_new(id: impl AsRef<str>, broker_url: impl Into<String>) -> Result<Self, Error> {
        let create_opts = CreateOptionsBuilder::new()
            .client_id(format!("fan/{}", id.as_ref()))
            .server_uri(broker_url)
            .finalize();

        Ok(Fan {
            id: id.as_ref().into(),
            client: AsyncClient::new(create_opts)?,
            state: Arc::new(Mutex::new(FanState {
                is_on: false,
                // TODO: for now we are putting the client here. Maybe to make it
                // more "modular" we can build the client alongside the device and
                // pass it in as a parameter.
                voltage: 240.0,
                speed: 1,
            })),
        })
    }

    pub fn turn_on(&mut self) {
        self.state.lock().is_on = true;
        info!(?self.id, "Turning on fan");
    }

    pub fn turn_off(&mut self) {
        self.state.lock().is_on = false;
        info!(?self.id, "Turning off fan");
    }

    pub fn set_speed(&mut self, speed: u8) {
        let mut state = self.state.lock();
        if state.is_on {
            state.speed = speed;
            info!("Setting speed");
        }
        warn!(
            is_on = state.is_on,
            "Cannot set the speed of a fan that is turned off"
        )
    }

    pub async fn publish_status(&self) -> Result<(), Error> {
        // TODO: each device publishes to a hardcoded topic. See if we can
        // "standardize" it
        let status = {
            let lock = self.state.lock();
            DeviceStatus::Fan(FanStatus {
                id: self.id.clone(),
                is_on: lock.is_on,
                speed: lock.speed,
                voltage: lock.voltage + thread_rng().gen_range(1.0..=3.0),
                timestamp: Utc::now(),
            })
        };

        let topic_name = format!("fan/{}/status", self.id);
        self.client
            .publish(Message::new_retained(
                topic_name,
                serde_json::to_string(&status)?,
                QOS_1,
            ))
            .await?;
        Ok(())
    }

    async fn process_payload(&mut self, msg: Message) -> Result<(), Error> {
        let payload = msg.payload();
        let Ok(command) = serde_json::from_slice::<FanCommand>(payload) else {
            warn!(?payload, "Invalid command received");
            // invalid payload is not the end of the world, hence no error.
            return Ok(());
        };
        match command {
            FanCommand::On => self.turn_on(),
            FanCommand::Off => self.turn_off(),
            FanCommand::Speed(speed) => self.set_speed(speed),
        }
        Ok(())
    }

    pub async fn handle_incoming(&mut self) -> Result<(), Error> {
        info!(?self.id, "Starting fan");

        // connect the client to the broker
        let connect_opts = ConnectOptionsBuilder::new_v5()
            .keep_alive_interval(Duration::from_secs(5))
            // if I am turned off, let others know that I am not available
            .will_message(Message::new_retained(
                format!("fan/{}/available", self.id),
                json!({
                    "is_available": false,
                })
                .to_string(),
                QOS_1,
            ))
            .finalize();
        self.client.connect(connect_opts).await?;

        // let others know that I am available now
        self.client
            .publish(Message::new_retained(
                format!("fan/{}/available", self.id),
                json!({
                    "is_available": true
                })
                .to_string(),
                QOS_1,
            ))
            .await?;

        // start a task to publish my status at regular intervals
        let self_clone = self.clone();
        let mut handle: JoinHandle<Result<(), Error>> = tokio::spawn(async move {
            loop {
                self_clone.publish_status().await?;
                sleep(Duration::from_secs(5)).await;
            }
        });

        // build a buffered stream to recieve messages but not overload the
        // memory
        let stream = self.client.get_stream(16);
        // listen for fan state and fan speed commands
        let _ = self
            .client
            .subscribe(format!("fan/{}/command", self.id), QOS_1)
            .await?;

        loop {
            select! {
                msg = stream.recv() => {
                    if let Ok(Some(msg)) = msg {
                        self.process_payload(msg).await?;
                    }
                }
                res = &mut handle => {
                    res??
                }
            }
        }
    }
}
