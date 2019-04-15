use crate::documents::DocMeta;
use crate::ids::{Entity, Id};
use crate::menu::Coffee;
use rand;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct Order {
    #[serde(flatten)]
    pub(super) meta: DocMeta<Order>,
    pub(super) coffee_id: Id<Coffee>,
}

impl Order {
    pub(super) fn for_coffee(coffee_id: Id<Coffee>) -> Self {
        let id = rand::random::<Id<Order>>();
        Order {
            meta: DocMeta::new_with_id(id),
            coffee_id: coffee_id,
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
