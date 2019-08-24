use serde::{Deserialize, Serialize};

use crate::menu::Drink;
use infra::documents::{DocMeta, HasMeta, MailBox};
use infra::ids::IdGen;
use infra::ids::{Entity, Id};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct Order {
    #[serde(flatten)]
    pub(super) meta: DocMeta<Order>,
    #[serde(default)]
    pub(super) mbox: MailBox<OrderDst>,
    pub(super) drink_id: Id<Drink>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub(super) enum OrderDst {
    Barista,
}

impl Order {
    pub(super) fn for_drink(drink_id: Id<Drink>, idgen: &IdGen) -> Self {
        let id = idgen.generate();
        let mut mbox = MailBox::empty();
        mbox.send(OrderDst::Barista);
        let meta = DocMeta::new_with_id(id);

        Order {
            meta,
            mbox,
            drink_id,
        }
    }
}
impl Entity for Order {
    const PREFIX: &'static str = "order";
}

impl HasMeta<Order> for Order {
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
                OrderDst::Barista,
            }
        )
    }
}
