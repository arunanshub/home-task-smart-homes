pub mod bulb;
pub mod error;
pub mod fan;
pub mod tv;

use bulb::Bulb;
use error::Error;
use fan::Fan;
use tokio::{select, spawn};
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
        let mut fan = self.fan;
        let mut tv = self.tv;

        let mut bulb_handle = tokio::spawn(async move { bulb.handle_incoming().await });
        let mut fan_handle = spawn(async move { fan.handle_incoming().await });
        let mut tv_handle = spawn(async move { tv.handle_incoming().await });

        loop {
            select! {
                res = &mut fan_handle => { res?? }
                res = &mut bulb_handle => { res?? }
                res = &mut tv_handle => {res??}
            }
        }
    }
}
