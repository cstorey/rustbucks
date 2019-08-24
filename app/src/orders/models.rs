use serde::{Deserialize, Serialize};

use crate::documents::{DocMeta, HasMeta, MailBox};
use crate::menu::Drink;
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

        Order {
            meta: DocMeta::new_with_id(id),
            mbox: mbox,
            drink_id: drink_id,
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
    use super::*;
    use infra::ids::Id;
    use maplit::hashset;

    #[test]
    fn should_request_coffee_made_on_creation() {
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
