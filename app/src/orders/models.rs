use crate::documents::{DocMeta, MailBox};
use crate::ids::IdGen;
use crate::ids::{Entity, Id};
use crate::menu::Drink;

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

impl AsRef<DocMeta<Order>> for Order {
    fn as_ref(&self) -> &DocMeta<Order> {
        &self.meta
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ids::Id;
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
