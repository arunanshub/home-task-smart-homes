use crate::error::Error;
use crate::Bulb;
use crate::Fan;
use crate::TV;
use tokio::{select, spawn};

#[derive(Debug)]
pub struct Home {
    pub name: String,
    pub bulb: Bulb,
    pub fan: Fan,
    pub tv: TV,
}

impl Home {
    pub fn new(id: impl Into<String>, bulb: Bulb, fan: Fan, tv: TV) -> Self {
        Self {
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
