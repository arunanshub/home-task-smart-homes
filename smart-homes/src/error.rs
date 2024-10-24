#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to create MQTT client: {0}")]
    ClientCreation(#[from] paho_mqtt::Error),

    #[error("Failed to join tokio task: {0}")]
    JoinError(#[from] tokio::task::JoinError),

    #[error("Failed to serialize message: {0}")]
    SerializeError(#[from] serde_json::Error),
}
