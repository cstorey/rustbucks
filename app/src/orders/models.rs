use std::cmp::Eq;
use std::collections::HashSet;
use std::hash::Hash;

use crate::documents::DocMeta;
use crate::ids::{Entity, Id};
use crate::menu::Drink;
use rand;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct Order {
    #[serde(flatten)]
    pub(super) meta: DocMeta<Order>,
    pub(super) mbox: MailBox<OrderDst>,
    pub(super) drink_id: Id<Drink>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub(super) enum OrderDst {
    Barista,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct MailBox<A: Eq + Hash> {
    pub(super) outgoing: HashSet<A>,
}

impl Order {
    pub(super) fn for_drink(drink_id: Id<Drink>) -> Self {
        let id = rand::random::<Id<Order>>();
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

impl AsRef<DocMeta<Order>> for Order {
    fn as_ref(&self) -> &DocMeta<Order> {
        &self.meta
    }
}

impl<A: Hash + Eq> MailBox<A> {
    fn empty() -> Self {
        let outgoing = HashSet::new();

        MailBox { outgoing }
    }

    fn send(&mut self, msg: A) {
        self.outgoing.insert(msg);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ids::Id;

    #[test]
    fn should_request_coffee_made_on_creation() {
        let drink = Id::hashed(&"english breakfast");
        let order = Order::for_drink(drink);

        assert_eq!(
            order.mbox.outgoing,
            hashset! {
                OrderDst::Barista,
            }
        )
    }
}
