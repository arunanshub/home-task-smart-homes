pub mod bulb;
pub mod cli;
pub mod error;
pub mod fan;
pub mod home;
pub mod tv;

use bulb::{Bulb, BulbStatus};
use fan::{Fan, FanStatus};
use serde::{Deserialize, Serialize};
use tv::{TVStatus, TV};

// TODO: maybe consider using tagged enum and then publish status like this:
// DeviceStatus::Bulb(BulbStatus { ... })
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DeviceStatus {
    Bulb(BulbStatus),
    Fan(FanStatus),
    TV(TVStatus),
}
