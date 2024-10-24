use crate::{error::Error, DeviceStatus};
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
pub struct BulbState {
    pub is_on: bool,
    pub speed: u8,
    pub voltage: f32,
    pub color: (u8, u8, u8),
}

#[derive(Educe, Clone)]
#[educe(Debug)]
pub struct Bulb {
    #[educe(Debug(ignore))]
    client: AsyncClient,
    pub id: String,
    state: Arc<Mutex<BulbState>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulbStatus {
    pub id: String,
    pub is_on: bool,
    pub speed: u8,
    pub voltage: f32,
    pub color: (u8, u8, u8),
}

/// Commands that can be recieved by the bulb.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "cmd", content = "args", rename_all = "snake_case")]
pub enum BulbCommand {
    /// Turn on the bulb.
    On,
    /// Turn off the bulb.
    Off,
    /// Set colour
    Color((u8, u8, u8)),
}

impl Bulb {
    pub fn try_new(id: impl AsRef<str>, broker_url: impl Into<String>) -> Result<Self, Error> {
        let create_opts = CreateOptionsBuilder::new()
            .client_id(format!("bulb/{}", id.as_ref()))
            .server_uri(broker_url)
            .finalize();

        Ok(Bulb {
            id: id.as_ref().into(),
            client: AsyncClient::new(create_opts)?,
            state: Arc::new(Mutex::new(BulbState {
                is_on: false,
                // TODO: for now we are putting the client here. Maybe to make it
                // more "modular" we can build the client alongside the bulb and
                // pass it in as a parameter.
                voltage: 240.0,
                speed: 1,
                color: (255, 255, 255),
            })),
        })
    }

    pub fn turn_on(&mut self) {
        self.state.lock().is_on = true;
        info!(self.id, "Turning on bulb");
    }

    pub fn turn_off(&mut self) {
        self.state.lock().is_on = false;
        info!(self.id, "Turning off bulb");
    }

    pub fn set_color(&mut self, color: (u8, u8, u8)) {
        self.state.lock().color = color;
        info!(self.id, ?color, "Changing color");
    }

    /// Publish the status of the device.
    pub async fn publish_status(&self) -> Result<(), Error> {
        let status = {
            let lock = self.state.lock();
            DeviceStatus::Bulb(BulbStatus {
                id: self.id.clone(),
                is_on: lock.is_on,
                speed: lock.speed,
                voltage: lock.voltage + thread_rng().gen_range(-5.0..5.0),
                color: lock.color,
            })
        };

        let topic_name = format!("bulb/{}/status", self.id);
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
        let Ok(command) = serde_json::from_slice::<BulbCommand>(payload) else {
            let payload_str = &*msg.payload_str();
            warn!(?payload, payload_str, "Invalid command received");
            // invalid payload is not the end of the world, hence no error.
            return Ok(());
        };
        match command {
            BulbCommand::On => self.turn_on(),
            BulbCommand::Off => self.turn_off(),
            BulbCommand::Color(v) => self.set_color(v),
        }
        Ok(())
    }

    pub async fn handle_incoming(&mut self) -> Result<(), Error> {
        info!(?self.id, "Starting bulb");

        // connect the client to the broker
        let connect_opts = ConnectOptionsBuilder::new_v5()
            .keep_alive_interval(Duration::from_secs(5))
            // if I am turned off, let others know that I am not available
            .will_message(Message::new_retained(
                format!("bulb/{}/available", self.id),
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
                format!("bulb/{}/available", self.id),
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
        // listen for bulb state
        let _ = self
            .client
            .subscribe(format!("bulb/{}/command", self.id), QOS_1)
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
