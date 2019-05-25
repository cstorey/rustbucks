use crate::documents::{DocMeta, MailBox};
use crate::ids::{Entity, Id};
use crate::menu::Drink;
use rand;

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
