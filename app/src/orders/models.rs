use crate::documents::DocMeta;
use crate::ids::{Entity, Id};
use crate::menu::Drink;
use rand;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct Order {
    #[serde(flatten)]
    pub(super) meta: DocMeta<Order>,
    pub(super) drink_id: Id<Drink>,
}

impl Order {
    pub(super) fn for_drink(drink_id: Id<Drink>) -> Self {
        let id = rand::random::<Id<Order>>();
        Order {
            meta: DocMeta::new_with_id(id),
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
