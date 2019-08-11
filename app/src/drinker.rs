use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::menu;
use infra::documents::{DocMeta, HasMeta};
use infra::ids::{Entity, Id, IdGen};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Drinker {
    #[serde(flatten)]
    pub(crate) meta: DocMeta<Drinker>,
    received_drinks: HashSet<Id<menu::Drink>>,
}

impl Drinker {
    pub fn incarnate(idgen: &IdGen) -> Self {
        let id = idgen.generate();
        let meta = DocMeta::new_with_id(id);
        let received_drinks = HashSet::new();
        Drinker {
            meta,
            received_drinks,
        }
    }

    #[cfg(test)]
    pub fn has_drink(&self, drink_id: Id<menu::Drink>) -> bool {
        self.received_drinks.contains(&drink_id)
    }

    #[cfg(test)]
    pub fn deliver_drink(&mut self, drink_id: Id<menu::Drink>) {
        self.received_drinks.insert(drink_id);
    }
}

impl Entity for Drinker {
    const PREFIX: &'static str = "drinker";
}

impl HasMeta for Drinker {
    fn meta(&self) -> &DocMeta<Drinker> {
        &self.meta
    }
    fn meta_mut(&mut self) -> &mut DocMeta<Drinker> {
        &mut self.meta
    }
}
impl AsMut<DocMeta<Drinker>> for Drinker {
    fn as_mut(&mut self) -> &mut DocMeta<Drinker> {
        &mut self.meta
    }
}
