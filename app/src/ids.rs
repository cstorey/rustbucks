use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use failure::Error;
use hybrid_clocks::{Clock, Timestamp, WallMS, WallMST};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug)]
pub struct Id<T> {
    stamp: Timestamp<WallMST>,
    random: u32,
    phantom: PhantomData<T>,
}

#[derive(Debug, Clone, Fail)]
enum IdParseError {
    InvalidPrefix,
    Unparseable,
}

pub trait Entity {
    const PREFIX: &'static str;
}

#[derive(Debug, Clone)]
pub struct IdGen {
    clock: Arc<Mutex<Clock<WallMS>>>,
}

const DIVIDER: &str = ".";

impl<T> Id<T> {
    /// Returns a id nominally at time zero, but with a random portion derived
    /// from the given entity.
    pub fn hashed<H: Hash>(entity: H) -> Self {
        let zero = time::Timespec::new(0, 0);
        let stamp = Timestamp {
            epoch: 0,
            time: WallMST::from_timespec(zero),
            count: 0,
        };

        let mut h = siphasher::sip::SipHasher24::new_with_keys(0, 0);
        entity.hash(&mut h);
        let random = h.finish() as u32;

        let phantom = PhantomData;
        Id {
            random,
            stamp,
            phantom,
        }
    }
}

impl IdGen {
    pub fn new() -> Self {
        let clock = Arc::new(Mutex::new(Clock::wall_ms()));
        IdGen { clock }
    }

    pub fn generate<T>(&self) -> Id<T> {
        let stamp = self.clock.lock().expect("clock lock").now();
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
        let mut off = 0;
        let (i, epoch) = vlq::decode_slice(&bytes[off..]);
        off += i;
        let (i, time_ms) = vlq::decode_slice(&bytes[off..]);
        off += i;
        let (i, count) = vlq::decode_slice(&bytes[off..]);
        off += i;
        let (_, random) = vlq::decode_slice(&bytes[off..]);

        let time = WallMST::of_u64(time_ms);
        let stamp = Timestamp { epoch, time, count };
        let phantom = PhantomData;

        Id {
            stamp,
            random,
            phantom,
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = [0u8; 20];
        let mut off = 0;

        off += vlq::encode_slice(self.stamp.epoch, &mut bytes[off..]);
        off += vlq::encode_slice(self.stamp.time.as_u64(), &mut bytes[off..]);
        off += vlq::encode_slice(self.stamp.count, &mut bytes[off..]);
        off += vlq::encode_slice(self.random, &mut bytes[off..]);
        bytes[..off].to_vec()
    }
}

const ENCODED_BARE_ID_LEN: usize = 22 + 5;

impl<T: Entity> fmt::Display for Id<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let mut buf = [0u8; ENCODED_BARE_ID_LEN];
        let sz = base64::encode_config_slice(&self.to_bytes(), base64::URL_SAFE_NO_PAD, &mut buf);

        write!(
            fmt,
            "{}{}{}",
            T::PREFIX,
            DIVIDER,
            String::from_utf8_lossy(&buf[..sz])
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

        let mut bytes = [0u8; 16 + 4];
        let sz = base64::decode_config_slice(b64, base64::URL_SAFE_NO_PAD, &mut bytes)?;

        return Ok(Self::from_bytes(&bytes[..sz]));
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
        let s = "wrongy-yxdgMe3dIHOX4NvCH90t4w";

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
        let s = "wrong-yxdgMe3dIHOX4NvCH90t4w";

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
        let s = "canary#yxdgMe3dIHOX4NvCH90t4w";
        let result = s.parse::<Id<Canary>>();

        assert!(
            result.is_err(),
            "Parsing {:?} should return error; got {:?}",
            s,
            result,
        )
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
