use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::menu;
use crate::product::Product;
use infra::documents::{DocMeta, HasMeta};
use infra::ids::{Entity, Id, IdGen};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Drinker {
    #[serde(flatten)]
    pub(crate) meta: DocMeta<Drinker>,
    pub(crate) received_drinks: HashSet<Id<Product>>,
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
