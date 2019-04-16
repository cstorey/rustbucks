use std::marker::PhantomData;
use std::cmp::Eq;
use std::collections::HashSet;
use std::hash::Hash;

use failure::Error;

use ids::{Entity, Id};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default, Hash)]
pub struct Version(String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(bound = "T: Entity")]
pub struct DocMeta<T> {
    #[serde(rename = "_id")]
    pub id: Id<T>,
    #[serde(rename = "_version")]
    pub version: Version,
    #[serde(skip)]
    pub _phantom: PhantomData<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct MailBox<A: Eq + Hash> {
    pub(super) outgoing: HashSet<A>,
}

impl<T> Default for DocMeta<T> {
    fn default() -> Self {
        let id = Default::default();
        let version = Default::default();
        let _phantom = Default::default();
        DocMeta {
            id,
            version,
            _phantom,
        }
    }
}

impl<T> DocMeta<T> {
    pub(crate) fn new_with_id(id: Id<T>) -> Self {
        DocMeta {
            id,
            ..Default::default()
        }
    }
}

impl std::str::FromStr for Version {
    type Err = Error;
    fn from_str(val: &str) -> Result<Self, Error> {
        let version = val.to_string();
        Ok(Version(version))
    }
}


impl<A: Hash + Eq> MailBox<A> {
    pub(crate) fn empty() -> Self {
        let outgoing = HashSet::new();

        MailBox { outgoing }
    }

    pub(crate) fn send(&mut self, msg: A) {
        self.outgoing.insert(msg);
    }
}

impl<A: Eq + Hash> Default for MailBox<A> {
    fn default() -> Self {
        Self::empty()
    }
}
