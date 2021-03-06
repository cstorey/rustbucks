use serde::{Deserialize, Serialize};

use crate::menu::Drink;
use infra::documents::{DocMeta, HasMeta, MailBox};
use infra::ids::{Entity, Id};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    #[serde(flatten)]
    pub(super) meta: DocMeta<Order>,
    #[serde(flatten)]
    pub(super) mbox: MailBox<OrderMsg>,
    pub(super) drink_id: Id<Drink>,
    #[serde(default)]
    pub(crate) is_made: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub(super) enum OrderMsg {
    DrinkRequest(Id<Drink>, Id<Order>),
}

impl Order {
    pub(super) fn for_drink(drink_id: Id<Drink>, id: Id<Self>) -> Self {
        let mut mbox = MailBox::empty();
        let meta = DocMeta::new_with_id(id);
        let is_made = false;

        mbox.send(OrderMsg::DrinkRequest(drink_id, id));

        Order {
            meta,
            mbox,
            drink_id,
            is_made,
        }
    }

    pub(crate) fn mark_fulfilled(&mut self) {
        self.is_made = true
    }
}

impl Entity for Order {
    const PREFIX: &'static str = "order";
}

impl HasMeta for Order {
    fn meta(&self) -> &DocMeta<Self> {
        &self.meta
    }
    fn meta_mut(&mut self) -> &mut DocMeta<Self> {
        &mut self.meta
    }
}

#[cfg(test)]
mod test {
    #[test]
    #[cfg(todo)]
    fn should_request_coffee_made_on_creation() {
        use super::*;
        use infra::ids::Id;
        use maplit::hashset;

        let drink = Id::hashed(&"english breakfast");
        let idgen = IdGen::new();
        let order = Order::for_drink(drink, &idgen);

        assert_eq!(
            order.mbox.outgoing,
            hashset! {
                OrderMsg::Barista,
            }
        )
    }
}
