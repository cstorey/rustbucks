use std::collections::BTreeSet;

use crate::documents::DocMeta;
use crate::ids::Entity;
use crate::ids::Id;

#[derive(Deserialize, Serialize, Debug, Clone, Hash)]
pub struct Drink {
    #[serde(flatten)]
    pub(super) meta: DocMeta<Drink>,
    pub(super) name: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DrinkList {
    #[serde(flatten)]
    pub(super) meta: DocMeta<DrinkList>,
    pub(super) drinks: BTreeSet<Id<Drink>>,
}

impl Drink {
    pub(super) fn new(id: Id<Drink>, name: &str) -> Self {
        let meta = DocMeta::new_with_id(id);
        let name = name.to_string();
        Drink { meta, name }
    }
}

impl DrinkList {
    pub(super) fn new(id: Id<DrinkList>) -> Self {
        let meta = DocMeta::new_with_id(id);
        let drinks = BTreeSet::new();
        DrinkList { meta, drinks }
    }
    pub(super) fn id() -> Id<DrinkList> {
        Id::hashed("DrinkList")
    }
}

impl Entity for Drink {
    const PREFIX: &'static str = "drink";
}

impl AsRef<DocMeta<Drink>> for Drink {
    fn as_ref(&self) -> &DocMeta<Drink> {
        &self.meta
    }
}
impl Entity for DrinkList {
    const PREFIX: &'static str = "drink_list";
}

impl AsRef<DocMeta<DrinkList>> for DrinkList {
    fn as_ref(&self) -> &DocMeta<DrinkList> {
        &self.meta
    }
}
