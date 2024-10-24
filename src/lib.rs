pub mod bulb;
pub mod cli;
pub mod error;
pub mod fan;
pub mod home;
pub mod http_api;
pub mod tv;

use bulb::{Bulb, BulbStatus};
use fan::{Fan, FanStatus};
use serde::{Deserialize, Serialize};
use tv::{TVStatus, TV};

// NOTE: using tagged enum so that it can be consumed in a more meaningful way
// by other clients outside the rust world.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "status", rename_all = "snake_case")]
pub enum DeviceStatus {
    Bulb(BulbStatus),
    Fan(FanStatus),
    #[serde(rename = "tv")]
    TV(TVStatus),
}
