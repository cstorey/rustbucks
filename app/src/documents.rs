use std::cmp::Eq;
use std::collections::HashSet;
use std::hash::Hash;
use std::marker::PhantomData;

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
    #[serde(rename = "_outgoing")]
    pub(super) outgoing: HashSet<A>,
}

impl<T> DocMeta<T> {
    pub(crate) fn new_with_id(id: Id<T>) -> Self {
        let version = Version::default();
        let _phantom = PhantomData;
        DocMeta {
            id,
            version,
            _phantom,
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

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn document_messaging_scratch_pad() {
        #[derive(Debug, Default, Hash, PartialEq, Eq)]
        struct Message;
        struct Source {
            mbox: MailBox<Message>,
        }
        struct Dest {
            items: u64,
        };
        impl Source {
            fn provoke(&mut self) {
                self.mbox.send(Message);
            }
        }
        impl Dest {
            fn receive(&mut self, _: Message) {
                self.items += 1
            }
        }
        let mut src = Source {
            mbox: MailBox::default(),
        };
        let mut dst = Dest { items: 0 };

        src.provoke();

        // A miracle occurs!
        for msg in src.mbox.outgoing.drain() {
            println!("Message  {:?}", msg);
            // Handler
            dst.receive(msg);
        }

        // ... A miracle has now occurred. Honest.
        assert_eq!(dst.items, 1);
    }
}
