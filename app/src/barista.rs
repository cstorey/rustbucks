use anyhow::Result;
use log::*;
use r2d2::{self, Pool};

use infra::{ids::Id, persistence::Storage};

use crate::menu::Drink;
use crate::services::{Commandable, Request};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PrepareDrink {
    pub drink_id: Id<Drink>,
}

#[derive(Debug)]
pub struct Barista<M: r2d2::ManageConnection> {
    db: Pool<M>,
}

impl<M: r2d2::ManageConnection<Connection = D>, D: Storage + Send + 'static> Barista<M> {
    pub fn new(db: Pool<M>) -> Result<Self> {
        Ok(Barista { db })
    }
}

impl Request for PrepareDrink {
    type Resp = ();
}

impl<M: r2d2::ManageConnection<Connection = D>, D: Storage + Send + 'static>
    Commandable<PrepareDrink> for Barista<M>
{
    fn execute(&self, order: PrepareDrink) -> Result<()> {
        let PrepareDrink { drink_id } = order;
        info!("Preparing drink {}!", drink_id);
        Ok(())
    }
}
