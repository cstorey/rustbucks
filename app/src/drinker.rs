use serde::{Deserialize, Serialize};

use crate::ids::{Entity, IdGen};
use infra::documents::{DocMeta, HasMeta};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Drinker {
    #[serde(flatten)]
    pub(crate) meta: DocMeta<Drinker>,
}

impl Drinker {
    pub fn incarnate(idgen: &IdGen) -> Self {
        let id = idgen.generate();
        let meta = DocMeta::new_with_id(id);
        Drinker { meta }
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
