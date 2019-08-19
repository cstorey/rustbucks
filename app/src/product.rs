use serde::{Deserialize, Serialize};

use crate::ids::{Entity, Id};
use crate::menu::Drink;
use infra::documents::{DocMeta, HasMeta};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    #[serde(flatten)]
    pub(crate) meta: DocMeta<Product>,
    pub(crate) recipie: Id<Drink>,
}

impl Entity for Product {
    const PREFIX: &'static str = "product";
}

impl HasMeta for Product {
    fn meta(&self) -> &DocMeta<Product> {
        &self.meta
    }
    fn meta_mut(&mut self) -> &mut DocMeta<Product> {
        &mut self.meta
    }
}
