use anyhow::Result;
use log::*;
use r2d2::Pool;

use crate::{
    menu::Drink,
    services::{Commandable, Request},
};
use infra::{
    ids::{Id, IdGen},
    persistence::Storage,
};

mod models;

use models::*;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlaceOrder {
    pub drink_id: Id<Drink>,
}

#[derive(Debug)]
pub struct Orders<M: r2d2::ManageConnection> {
    db: Pool<M>,
    idgen: IdGen,
}

impl<M: r2d2::ManageConnection<Connection = D>, D: Storage + Send + 'static> Orders<M> {
    pub fn new(db: Pool<M>, idgen: IdGen) -> Result<Self> {
        Ok(Orders { db, idgen })
    }
}

impl Request for PlaceOrder {
    type Resp = Id<Order>;
}

impl<M: r2d2::ManageConnection<Connection = D>, D: Storage + Send + 'static> Commandable<PlaceOrder>
    for &Orders<M>
{
    fn execute(self, order: PlaceOrder) -> Result<Id<Order>> {
        let docs = self.db.get()?;
        let mut order = Order::for_drink(order.drink_id, self.idgen.generate());
        docs.save(&mut order)?;
        debug!("Saved {:?}", order);
        Ok(order.meta.id)
    }
}
