use std::collections::BTreeSet;

use crate::documents::DocMeta;
use crate::ids::Entity;
use crate::ids::Id;

#[derive(Deserialize, Serialize, Debug, Clone, Hash, Default)]
pub struct Coffee {
    #[serde(flatten)]
    pub(super) meta: DocMeta<Coffee>,
    pub(super) name: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct CoffeeList {
    #[serde(flatten)]
    pub(super) meta: DocMeta<CoffeeList>,
    pub(super) drinks: BTreeSet<Id<Coffee>>,
}

impl CoffeeList {
    pub(super) fn id() -> Id<CoffeeList> {
        Id::hashed(&"CoffeeList")
    }
}

impl Entity for Coffee {
    const PREFIX: &'static str = "coffee";
}

impl AsRef<DocMeta<Coffee>> for Coffee {
    fn as_ref(&self) -> &DocMeta<Coffee> {
        &self.meta
    }
}
impl Entity for CoffeeList {
    const PREFIX: &'static str = "coffee_list";
}

impl AsRef<DocMeta<CoffeeList>> for CoffeeList {
    fn as_ref(&self) -> &DocMeta<CoffeeList> {
        &self.meta
    }
}
