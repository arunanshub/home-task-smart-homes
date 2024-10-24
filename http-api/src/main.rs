use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use paho_mqtt::{AsyncClient, ConnectOptionsBuilder};
use smart_homes::DeviceStatus;
use std::time::Duration;

async fn get_bulb_info(
    Path(home_id): Path<u32>,
    State(state): State<SharedState>,
) -> Json<DeviceStatus> {
    let topic = format!("bulb/home/{}/status", home_id);
    let mut client = state.client;
    let _ = client.subscribe(topic, 1).await.unwrap();

    let buf = client.get_stream(16);
    let Ok(Some(msg)) = buf.recv().await else {
        // TODO: add retry/error handling
        panic!()
    };
    return Json(serde_json::from_slice(msg.payload()).unwrap());
}

async fn get_fan_info(
    Path(house_id): Path<u32>,
    State(state): State<SharedState>,
) -> Json<DeviceStatus> {
    let topic = format!("fan/home/{}/status", house_id);
    let mut client = state.client;

    let _ = client.subscribe(topic, 1).await.unwrap();
    let buf = client.get_stream(16);
    let Ok(Some(msg)) = buf.recv().await else {
        // TODO: add retry/error handling
        panic!()
    };
    return Json(serde_json::from_slice(msg.payload()).unwrap());
}

async fn get_tv_info(
    Path(house_id): Path<u32>,
    State(state): State<SharedState>,
) -> Json<DeviceStatus> {
    let topic = format!("tv/home/{}/status", house_id);
    let mut client = state.client;

    let _ = client
        .subscribe_with_options(topic, 1, None, None)
        .await
        .unwrap();
    let buf = client.get_stream(16);
    let Ok(Some(msg)) = buf.recv().await else {
        // TODO: add retry/error handling
        panic!()
    };
    return Json(serde_json::from_slice(msg.payload()).unwrap());
}

#[derive(Clone)]
struct SharedState {
    client: AsyncClient,
}

#[tokio::main]
async fn main() {
    let client = AsyncClient::new("tcp://localhost:1883").unwrap();
    client
        .connect(
            ConnectOptionsBuilder::new_v5()
                .connect_timeout(Duration::from_secs(5))
                .finalize(),
        )
        .await
        .unwrap();

    let app = Router::new()
        .route("/house/:house_id/bulb/status", get(get_bulb_info))
        .route("/house/:house_id/fan/status", get(get_fan_info))
        .route("/house/:house_id/tv/status", get(get_tv_info))
        .with_state(SharedState { client });

    let listener = tokio::net::TcpListener::bind("localhost:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
