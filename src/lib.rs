pub mod bulb;
pub mod error;
pub mod fan;
pub mod tv;

use bulb::Bulb;
use error::Error;
use fan::Fan;
use tokio::select;
use tracing::error;
use tv::TV;

#[derive(Debug)]
pub struct House {
    pub name: String,
    pub bulb: Bulb,
    pub fan: Fan,
    pub tv: TV,
}

pub enum HouseCommand {
    // TurnOnAll,
    // TurnOffAll,
}

impl House {
    pub fn new(id: impl Into<String>, bulb: Bulb, fan: Fan, tv: TV) -> Self {
        House {
            name: id.into(),
            bulb,
            fan,
            tv,
        }
    }

    pub async fn handle_incoming(self) -> Result<(), Error> {
        let mut bulb = self.bulb;
        let mut bulb_handle = tokio::spawn(async move { bulb.handle_incoming().await });

        loop {
            select! {
                res = &mut bulb_handle => {
                    if let Err(err) = res {
                        error!(?err);
                    }
                }
            }
        }
    }
}
