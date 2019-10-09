use anyhow::Result;
use log::*;
use r2d2::Pool;

use crate::{
    barista::PrepareDrink,
    menu::Drink,
    services::{Commandable, Request},
};
use infra::{
    ids::{Id, IdGen},
    persistence::{Storage, StoragePending},
};

mod models;

use models::*;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlaceOrder {
    pub drink_id: Id<Drink>,
}

#[derive(Debug)]
pub struct Orders<M: r2d2::ManageConnection, B> {
    db: Pool<M>,
    idgen: IdGen,
    barista: B,
}

impl<
        M: r2d2::ManageConnection<Connection = D>,
        D: Storage + StoragePending + Send + 'static,
        B: Commandable<PrepareDrink>,
    > Orders<M, B>
{
    pub fn new(db: Pool<M>, idgen: IdGen, barista: B) -> Result<Self> {
        Ok(Orders { db, idgen, barista })
    }

    pub fn process_action(&self) -> Result<()> {
        let conn = self.db.get()?;
        if let Some(mut doc) = conn.load_next_unsent::<Order>()? {
            info!("Found pending document: {:?}", doc);
            while let Some(act) = doc.mbox.take_one() {
                self.handle_order_action(act)?;
                conn.save(&mut doc)?;
            }
        }
        Ok(())
    }

    fn handle_order_action(&self, action: OrderMsg) -> Result<()> {
        info!("Action: {:?}", action);
        match action {
            OrderMsg::DrinkRequest(item_id, order_id) => {
                info!("Drink req: item:{}; order:{}", item_id, order_id);
                self.barista.execute(PrepareDrink { drink_id: item_id })?
            }
        };
        Ok(())
    }
}

impl Request for PlaceOrder {
    type Resp = Id<Order>;
}

impl<M: r2d2::ManageConnection<Connection = D>, D: Storage + Send + 'static, B>
    Commandable<PlaceOrder> for Orders<M, B>
{
    fn execute(&self, order: PlaceOrder) -> Result<Id<Order>> {
        let docs = self.db.get()?;
        let mut order = Order::for_drink(order.drink_id, self.idgen.generate());
        docs.save(&mut order)?;
        debug!("Saved {:?}", order);
        Ok(order.meta.id)
    }
}
