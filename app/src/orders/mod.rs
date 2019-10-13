use anyhow::Result;
use log::*;
use r2d2::Pool;

use crate::{
    barista::PrepareDrink,
    menu::Drink,
    services::{Commandable, Queryable, Request},
};
use infra::{
    ids::{Id, IdGen},
    persistence::{Storage, StoragePending},
};

mod models;

pub use models::Order;
use models::*;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlaceOrder {
    pub drink_id: Id<Drink>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FulfillDrink {
    pub order_id: Id<Order>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct QueryOrder {
    pub order_id: Id<Order>,
}
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct OrderStatus {
    pub order_id: Id<Order>,
    pub is_made: bool,
}

#[derive(Debug)]
pub struct Orders<M: r2d2::ManageConnection> {
    db: Pool<M>,
    idgen: IdGen,
}

pub struct OrderWorker<M: r2d2::ManageConnection, B> {
    db: Pool<M>,
    barista: B,
}

impl<M: r2d2::ManageConnection<Connection = D>, D: Storage + StoragePending + Send + 'static>
    Orders<M>
{
    pub fn new(db: Pool<M>, idgen: IdGen) -> Result<Self> {
        Ok(Orders { db, idgen })
    }
}

impl<
        M: r2d2::ManageConnection<Connection = D>,
        D: Storage + StoragePending + Send + 'static,
        B: Commandable<PrepareDrink>,
    > OrderWorker<M, B>
{
    pub fn new(db: Pool<M>, barista: B) -> Result<Self> {
        Ok(OrderWorker { db, barista })
    }

    pub fn process_action(&self) -> Result<()> {
        self.db.get()?.subscribe(|mut doc: Order| {
            info!("Found pending document: {:?}", doc);
            while let Some(act) = doc.mbox.take_one() {
                self.handle_order_action(act)?;
                self.db.get()?.save(&mut doc)?;
            }
            Ok(())
        })?;
        Ok(())
    }

    fn handle_order_action(&self, action: OrderMsg) -> Result<()> {
        info!("Action: {:?}", action);
        match action {
            OrderMsg::DrinkRequest(drink_id, order_id) => {
                info!("Drink req: item:{}; order:{}", drink_id, order_id);
                self.barista.execute(PrepareDrink { drink_id, order_id })?
            }
        };
        Ok(())
    }
}

impl Request for PlaceOrder {
    type Resp = Id<Order>;
}

impl Request for FulfillDrink {
    type Resp = ();
}

impl Request for QueryOrder {
    type Resp = OrderStatus;
}

impl<M: r2d2::ManageConnection<Connection = D>, D: Storage + Send + 'static> Commandable<PlaceOrder>
    for Orders<M>
{
    fn execute(&self, order: PlaceOrder) -> Result<Id<Order>> {
        let docs = self.db.get()?;
        let mut order = Order::for_drink(order.drink_id, self.idgen.generate());
        docs.save(&mut order)?;
        debug!("Saved {:?}", order);
        info!("Order placed: {}", order.meta.id);
        Ok(order.meta.id)
    }
}
impl<M: r2d2::ManageConnection<Connection = D>, D: Storage + Send + 'static>
    Commandable<FulfillDrink> for Orders<M>
{
    fn execute(&self, FulfillDrink { order_id }: FulfillDrink) -> Result<()> {
        let docs = self.db.get()?;
        let mut order = docs
            .load(&order_id)?
            .ok_or_else(|| anyhow::anyhow!("Order not found? id:{}", order_id))?;

        order.mark_fulfilled();
        docs.save(&mut order)?;
        Ok(())
    }
}
impl<M: r2d2::ManageConnection<Connection = D>, D: Storage + Send + 'static> Queryable<QueryOrder>
    for Orders<M>
{
    fn query(&self, QueryOrder { order_id }: QueryOrder) -> Result<OrderStatus> {
        let docs = self.db.get()?;
        let order = docs
            .load(&order_id)?
            .ok_or_else(|| anyhow::anyhow!("Order not found? id:{}", order_id))?;
        let Order { is_made, .. } = order;

        let resp = OrderStatus { order_id, is_made };

        Ok(resp)
    }
}
