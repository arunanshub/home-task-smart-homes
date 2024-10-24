pub mod bulb;
pub mod error;
pub mod fan;

use bulb::Bulb;
use error::Error;
use tokio::select;
use tracing::error;

#[derive(Debug)]
pub struct House {
    pub name: String,
    pub bulb: bulb::Bulb,
}

pub enum HouseCommand {
    // TurnOnAll,
    // TurnOffAll,
}

impl House {
    pub fn new(id: impl Into<String>, bulb: Bulb) -> Self {
        House {
            name: id.into(),
            bulb,
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
