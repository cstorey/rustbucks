use failure::{bail, Fail};
use std::cmp::Ordering;
use std::convert::TryInto;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::time::SystemTime;

use data_encoding::BASE32_DNSSEC;
use failure::Error;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

use crate::untyped_ids::UntypedId;

pub(crate) const ENCODED_BARE_ID_LEN: usize = 26;

#[derive(Debug)]
pub struct Id<T> {
    // Unix time in ms
    pub(crate) stamp: u64,
    pub(crate) random: u64,
    phantom: PhantomData<T>,
}

#[derive(Debug, Clone, Fail)]
pub enum IdParseError {
    InvalidPrefix,
    Unparseable,
}

pub trait Entity {
    const PREFIX: &'static str;
}

#[derive(Debug, Clone)]
pub struct IdGen {}

const DIVIDER: &str = ".";

impl<T> Id<T> {
    /// Returns a id nominally at time zero, but with a random portion derived
    /// from the given entity.
    pub fn hashed<H: Hash>(entity: H) -> Self {
        let stamp = 0;

        let mut h = siphasher::sip::SipHasher24::new_with_keys(0, 0);
        entity.hash(&mut h);
        let random = h.finish();

        let phantom = PhantomData;
        Id {
            stamp,
            random,
            phantom,
        }
    }
}

impl IdGen {
    pub fn new() -> Self {
        IdGen {}
    }

    pub fn generate<T>(&self) -> Id<T> {
        let stamp_epoch = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("now");
        let stamp_s: u64 = stamp_epoch
            .as_secs()
            .checked_mul(1000)
            .expect("secs * 1000");
        let stamp_ms: u64 = stamp_epoch.subsec_millis().into();
        let stamp = stamp_s + stamp_ms;
        let random = rand::random();
        let phantom = PhantomData;

        Id {
            random,
            stamp,
            phantom,
        }
    }
}

impl<T> Id<T> {
    fn from_bytes(bytes: &[u8]) -> Self {
        let stamp = u64::from_be_bytes(bytes[0..8].try_into().expect("stamp bytes"));
        let random = u64::from_be_bytes(bytes[8..8 + 8].try_into().expect("random bytes"));

        let phantom = PhantomData;

        Id {
            stamp,
            random,
            phantom,
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(16);
        bytes.extend(&self.stamp.to_be_bytes());
        bytes.extend(&self.random.to_be_bytes());
        bytes
    }
}

impl<T: Entity> fmt::Display for Id<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let mut buf = [0u8; ENCODED_BARE_ID_LEN];
        BASE32_DNSSEC.encode_mut(&self.to_bytes(), &mut buf);

        write!(
            fmt,
            "{}{}{}",
            T::PREFIX,
            DIVIDER,
            String::from_utf8_lossy(&buf[..])
        )?;
        Ok(())
    }
}

impl<T: Entity> std::str::FromStr for Id<T> {
    type Err = Error;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let expected_length = T::PREFIX.len() + DIVIDER.len();
        if src.len() < expected_length {
            bail!(IdParseError::InvalidPrefix);
        };
        let (start, remainder) = src.split_at(T::PREFIX.len());
        if start != T::PREFIX {
            bail!(IdParseError::InvalidPrefix);
        }
        let (divider, b64) = remainder.split_at(DIVIDER.len());

        if divider != DIVIDER {
            bail!(IdParseError::Unparseable);
        }

        let mut bytes = [0u8; 16];
        BASE32_DNSSEC
            .decode_mut(b64.as_bytes(), &mut bytes)
            .map_err(|e| failure::format_err!("{:?}", e))?;

        return Ok(Self::from_bytes(&bytes[..]));
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.stamp == other.stamp && self.random == other.random
    }
}

impl<T> Eq for Id<T> {}

impl<T> PartialOrd for Id<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Id<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.stamp
            .cmp(&other.stamp)
            .then_with(|| self.random.cmp(&other.random))
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Id {
            stamp: self.stamp,
            random: self.random,
            phantom: self.phantom,
        }
    }
}

impl<T> Copy for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.stamp.hash(hasher);
        self.random.hash(hasher);
    }
}

impl<T: Entity> Serialize for Id<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de, T: Entity> Deserialize<'de> for Id<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct IdStrVisitor<T>(PhantomData<T>);
        impl<'vi, T: Entity> de::Visitor<'vi> for IdStrVisitor<T> {
            type Value = Id<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "an Id string")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Id<T>, E> {
                value.parse::<Id<T>>().map_err(E::custom)
            }
        }

        deserializer.deserialize_str(IdStrVisitor(PhantomData))
    }
}

impl fmt::Display for IdParseError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &IdParseError::InvalidPrefix => write!(fmt, "Invalid prefix"),
            &IdParseError::Unparseable => write!(fmt, "Unparseable Id"),
        }
    }
}

impl<T> From<UntypedId> for Id<T> {
    fn from(src: UntypedId) -> Self {
        Id {
            stamp: src.stamp,
            random: src.random,
            phantom: PhantomData,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json;

    #[derive(Debug)]
    struct Canary;

    impl Entity for Canary {
        const PREFIX: &'static str = "canary";
    }

    #[test]
    fn round_trips_via_to_from_str() {
        let id = Id::<Canary>::hashed(&"Hi!");
        let s = id.to_string();
        println!("String: {}", s);
        let id2 = s.parse::<Id<Canary>>().expect("parse id");
        assert_eq!(id, id2);
    }

    #[test]
    fn round_trips_via_to_from_str_now() {
        let id = IdGen::new().generate::<Canary>();
        let s = id.to_string();
        println!("String: {}", s);
        let id2 = s.parse::<Id<Canary>>().expect("parse id");
        assert_eq!(id, id2);
    }

    #[test]
    fn round_trips_via_serde_json() {
        let id = Id::<Canary>::hashed(&"boo");

        let json = serde_json::to_string(&id).expect("serde_json::to_string");
        println!("Json: {}", json);
        let id2 = serde_json::from_str(&json).expect("serde_json::from_str");
        assert_eq!(id, id2);
    }

    #[test]
    fn round_trips_via_untyped() {
        let id = Id::<Canary>::hashed(&"boo");

        let untyped: UntypedId = id.into();
        println!("untyped: {}", untyped);
        let id2: Id<Canary> = untyped.into();
        assert_eq!(id, id2);
    }

    #[test]
    fn serializes_to_string_like() {
        let id = Id::<Canary>::hashed(&"Hi!");

        let json = serde_json::to_string(&id).expect("serde_json::to_string");
        let s: String = serde_json::from_str(&json).expect("serde_json::from_str");
        assert_eq!(id.to_string(), s);
    }

    #[test]
    fn should_allow_random_generation() {
        let idgen = IdGen::new();
        let id = idgen.generate::<Canary>();
        let id2 = idgen.generate::<Canary>();

        assert_ne!(id, id2);
    }

    #[test]
    fn should_allow_ordering() {
        let idgen = IdGen::new();
        let id = idgen.generate::<Canary>();
        let mut id2 = idgen.generate::<Canary>();
        while id2 == id {
            id2 = idgen.generate::<Canary>();
        }

        assert!(id < id2 || id > id2);
    }

    #[test]
    fn to_string_should_be_prefixed_with_type_name() {
        let idgen = IdGen::new();
        let id = idgen.generate::<Canary>();

        let s = id.to_string();

        assert!(
            s.starts_with("canary"),
            "string: {:?} starts with {:?}",
            s,
            "canary"
        )
    }
    #[test]
    fn should_verify_has_correct_entity_prefix() {
        let s = "wrongy-0000000000001q5nnvfqq7krfo";

        let result = s.parse::<Id<Canary>>();

        assert!(
            result.is_err(),
            "Parsing {:?} should return error; got {:?}",
            s,
            result,
        )
    }

    #[test]
    fn should_yield_useful_error_when_invalid_prefix() {
        #[derive(Debug)]
        struct Long;
        impl Entity for Long {
            // Borrowed from https://en.wikipedia.org/wiki/Longest_word_in_English
            // We want it to be longer than the id string in total.
            const PREFIX: &'static str = "pseudopseudohypoparathyroidism";
        }
        let s = "wrong-0000000000001q5nnvfqq7krfo";

        let result = s.parse::<Id<Long>>();

        assert!(
            result.is_err(),
            "Parsing {:?} should return error; got {:?}",
            s,
            result,
        )
    }

    #[test]
    fn should_yield_useful_error_when_just_prefix() {
        let s = "canary";
        let result = s.parse::<Id<Canary>>();

        assert!(
            result.is_err(),
            "Parsing {:?} should return error; got {:?}",
            s,
            result,
        )
    }
    #[test]
    fn should_yield_useful_error_when_wrong_divider() {
        let s = "canary#0000000000001q5nnvfqq7krfo";
        let result = s.parse::<Id<Canary>>();

        assert!(
            result.is_err(),
            "Parsing {:?} should return error; got {:?}",
            s,
            result,
        )
    }
}
