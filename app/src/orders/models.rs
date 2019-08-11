use serde::{Deserialize, Serialize};

use crate::drinker::Drinker;
use crate::menu::Drink;
use infra::documents::{DocMeta, HasMeta, MailBox};
use infra::ids::IdGen;
use infra::ids::{Entity, Id};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    #[serde(flatten)]
    pub(crate) meta: DocMeta<Order>,
    #[serde(default, flatten)]
    pub(crate) mbox: MailBox<OrderDst>,
    pub(crate) drink_id: Id<Drink>,
    pub(crate) drinker_id: Id<Drinker>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub(crate) enum OrderDst {
    Barista(Id<Drinker>, Id<Drink>),
}

impl Order {
    pub fn for_drink(drink_id: Id<Drink>, drinker_id: Id<Drinker>, idgen: &IdGen) -> Self {
        let id = idgen.generate();
        let mbox = MailBox::empty();
        let meta = DocMeta::new_with_id(id);

        let mut me = Order {
            meta,
            mbox,
            drink_id,
            drinker_id,
        };
        me.mbox.send(OrderDst::Barista(drinker_id, drink_id));
        me
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
impl AsMut<DocMeta<Order>> for Order {
    fn as_mut(&mut self) -> &mut DocMeta<Order> {
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
        let drinker = idgen.generate::<Drinker>();
        let order = Order::for_drink(drink, drinker, &idgen);

        assert_eq!(
            order.mbox.outgoing,
            hashset! {
                OrderDst::Barista(drinker, drink),
            }
        )
    }
}
