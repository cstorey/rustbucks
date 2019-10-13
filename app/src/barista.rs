use anyhow::Result;
use log::*;
use r2d2::{self, Pool};
use serde::{Deserialize, Serialize};

use infra::{
    documents::{DocMeta, HasMeta, MailBox},
    ids::{Entity, Id},
    persistence::{Storage, StoragePending},
};

use crate::menu::Drink;
use crate::orders::FulfillDrink;
use crate::orders::Order;
use crate::services::{Commandable, Request};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PrepareDrink {
    pub drink_id: Id<Drink>,
    pub order_id: Id<Order>,
}

#[derive(Debug)]
pub struct Barista<M: r2d2::ManageConnection> {
    db: Pool<M>,
}
#[derive(Debug)]
pub struct BaristaWorker<M: r2d2::ManageConnection, O> {
    db: Pool<M>,
    orders: O,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrinkPreparation {
    #[serde(flatten)]
    pub(super) meta: DocMeta<DrinkPreparation>,
    #[serde(flatten)]
    pub(super) mbox: MailBox<PreparationMsg>,
    pub(super) drink_id: Id<Drink>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub(super) enum PreparationMsg {
    FulfillDrink(Id<Order>),
}

impl<M: r2d2::ManageConnection<Connection = D>, D: Storage + StoragePending + Send + 'static>
    Barista<M>
{
    pub fn new(db: Pool<M>) -> Result<Self> {
        Ok(Barista { db })
    }
}

impl<
        M: r2d2::ManageConnection<Connection = D>,
        D: Storage + StoragePending + Send + 'static,
        O: Commandable<FulfillDrink>,
    > BaristaWorker<M, O>
{
    pub fn new(db: Pool<M>, orders: O) -> Result<Self> {
        Ok(BaristaWorker { db, orders })
    }

    pub fn process_action(&self) -> Result<()> {
        self.db.get()?.subscribe(|mut doc: DrinkPreparation| {
            info!("Found pending document: {:?}", doc);
            while let Some(act) = doc.mbox.take_one() {
                self.handle_barista_action(act)?;
                self.db.get()?.save(&mut doc)?;
            }
            Ok(())
        })?;
        Ok(())
    }

    fn handle_barista_action(&self, action: PreparationMsg) -> Result<()> {
        info!("Action: {:?}", action);
        match action {
            PreparationMsg::FulfillDrink(order_id) => {
                info!("Fulfil drink: order:{}", order_id);
                self.orders.execute(FulfillDrink { order_id })?
            }
        };
        Ok(())
    }
}

impl Request for PrepareDrink {
    type Resp = ();
}

impl<M: r2d2::ManageConnection<Connection = D>, D: Storage + Send + 'static>
    Commandable<PrepareDrink> for Barista<M>
{
    fn execute(&self, order: PrepareDrink) -> Result<()> {
        let PrepareDrink { drink_id, order_id } = order;
        info!("Preparing drink {}!", drink_id);

        let mut mbox = MailBox::empty();
        let meta = DocMeta::new_with_id(order_id.untyped().typed());

        mbox.send(PreparationMsg::FulfillDrink(order_id));

        let mut prep = DrinkPreparation {
            meta,
            mbox,
            drink_id,
        };

        self.db.get()?.save(&mut prep)?;
        debug!("Saved {:?}", prep);

        Ok(())
    }
}

impl Entity for DrinkPreparation {
    const PREFIX: &'static str = "drink-preparation";
}

impl HasMeta for DrinkPreparation {
    fn meta(&self) -> &DocMeta<Self> {
        &self.meta
    }
    fn meta_mut(&mut self) -> &mut DocMeta<Self> {
        &mut self.meta
    }
}
