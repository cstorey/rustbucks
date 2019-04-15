use crate::documents::DocMeta;
use crate::ids::{Entity, Id};
use crate::menu::Coffee;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct Order {
    #[serde(flatten)]
    pub(super) meta: DocMeta<Order>,
    pub(super) coffee_id: Id<Coffee>,
}

impl Entity for Order {
    const PREFIX: &'static str = "order";
}

impl AsRef<DocMeta<Order>> for Order {
    fn as_ref(&self) -> &DocMeta<Order> {
        &self.meta
    }
}
