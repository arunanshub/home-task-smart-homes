use crate::{error::Error, DeviceStatus};
use chrono::{DateTime, Utc};
use educe::Educe;
use paho_mqtt::{AsyncClient, ConnectOptionsBuilder, CreateOptionsBuilder, Message, QOS_1};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{sync::Arc, time::Duration};
use tokio::{select, task::JoinHandle, time::sleep};
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct TVState {
    pub is_on: bool,
    pub channel: u16,
    pub volume: u8,
}

#[derive(Educe, Clone)]
#[educe(Debug)]
pub struct TV {
    #[educe(Debug(ignore))]
    client: AsyncClient,
    pub id: String,
    state: Arc<Mutex<TVState>>,
}

/// Holds the status report of the tv.
#[serde_with::serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TVStatus {
    pub id: String,
    pub is_on: bool,
    pub channel: u16,
    pub volume: u8,
    pub is_muted: bool,
    #[serde_as(as = "serde_with::TimestampSeconds<i64, serde_with::formats::Flexible>")]
    pub timestamp: DateTime<Utc>,
}

/// Commands that can be recieved by the tv.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "cmd", content = "args", rename_all = "snake_case")]
pub enum TVCommand {
    /// Turn on the tv.
    On,
    /// Turn off the tv.
    Off,
    /// Set volume
    Volume(u8),
    /// Mute
    Mute,
    /// Set channel
    Channel(u16),
}

impl TV {
    pub fn try_new(id: impl AsRef<str>, broker_url: impl Into<String>) -> Result<Self, Error> {
        let create_opts = CreateOptionsBuilder::new()
            .client_id(format!("tv/{}", id.as_ref()))
            .server_uri(broker_url)
            .finalize();

        Ok(TV {
            id: id.as_ref().into(),
            // TODO: for now we are putting the client here. Maybe to make it
            // more "modular" we can build the client alongside the tv and
            // pass it in as a parameter.
            client: AsyncClient::new(create_opts)?,
            state: Arc::new(Mutex::new(TVState {
                is_on: false,
                channel: 1,
                volume: 10,
            })),
        })
    }

    pub fn turn_on(&mut self) {
        self.state.lock().is_on = true;
        info!(self.id, "Turning on tv");
    }

    pub fn turn_off(&mut self) {
        self.state.lock().is_on = false;
        info!(self.id, "Turning off tv");
    }

    pub fn set_channel(&mut self, channel: u16) {
        self.state.lock().channel = channel;
        info!(channel, self.id, "Changing channel");
    }

    pub fn set_volume(&mut self, volume: u8) {
        self.state.lock().volume = volume;
        info!(volume, self.id, "Changing channel");
    }

    /// Publish the status of the device.
    pub async fn publish_status(&self) -> Result<(), Error> {
        let data = {
            let lock = self.state.lock();
            DeviceStatus::TV(TVStatus {
                channel: lock.channel,
                id: self.id.clone(),
                is_on: lock.is_on,
                volume: lock.volume,
                is_muted: lock.volume == 0,
                timestamp: Utc::now(),
            })
        };

        let topic_name = format!("tv/{}/status", self.id);
        self.client
            .publish(Message::new_retained(
                topic_name,
                serde_json::to_string(&data)?,
                QOS_1,
            ))
            .await?;
        Ok(())
    }

    async fn process_payload(&mut self, msg: Message) -> Result<(), Error> {
        let payload = msg.payload();
        let Ok(command) = serde_json::from_slice::<TVCommand>(payload) else {
            let payload_str = &*msg.payload_str();
            warn!(?payload, payload_str, "Invalid command received");
            // invalid payload is not the end of the world, hence no error.
            return Ok(());
        };
        match command {
            TVCommand::On => self.turn_on(),
            TVCommand::Off => self.turn_off(),
            TVCommand::Channel(v) => self.set_channel(v),
            TVCommand::Mute => self.set_volume(0),
            TVCommand::Volume(v) => self.set_volume(v),
        }
        Ok(())
    }

    pub async fn handle_incoming(&mut self) -> Result<(), Error> {
        info!(?self.id, "Starting tv");

        // connect the client to the broker
        let connect_opts = ConnectOptionsBuilder::new_v5()
            .keep_alive_interval(Duration::from_secs(5))
            // if I am turned off, let others know that I am not available
            .will_message(Message::new_retained(
                format!("tv/{}/available", self.id),
                json!({
                    "is_available": false,
                })
                .to_string(),
                QOS_1,
            ))
            .finalize();

        self.client.connect(connect_opts).await?;
        info!(?self.id, "connected");

        // let others know that I am available now
        self.client
            .publish(Message::new_retained(
                format!("tv/{}/available", self.id),
                json!({
                    "is_available": true
                })
                .to_string(),
                QOS_1,
            ))
            .await?;

        // start a task to publish my status at regular intervals
        let self_clone = self.clone();
        let mut status_pub_task: JoinHandle<Result<(), Error>> = tokio::spawn(async move {
            loop {
                self_clone.publish_status().await?;
                sleep(Duration::from_secs(5)).await;
            }
        });

        // build a buffered stream to recieve messages but not overload the
        // memory
        let stream = self.client.get_stream(16);
        // listen for tv state
        let _ = self
            .client
            .subscribe(format!("tv/{}/command", self.id), QOS_1)
            .await?;

        loop {
            select! {
                msg = stream.recv() => {
                    if let Ok(Some(msg)) = msg {
                        self.process_payload(msg).await?;
                    }
                }
                res = &mut status_pub_task => {
                    res??
                }
            }
        }
    }
}
