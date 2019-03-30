use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::io;
use std::marker::PhantomData;

use byteorder::{BigEndian, WriteBytesExt};
use failure::Error;
use hex_slice::AsHex;
use rand::distributions::{Distribution, Standard};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Hash)]
pub struct Id<T> {
    val: [u8; 16],
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

const DIVIDER: &str = "-";

impl<T> Id<T> {
    pub fn hashed<H: Hash>(entity: &H) -> Self {
        let mut val = [0u8; 16];
        {
            let mut cursor = io::Cursor::new(&mut val as &mut [u8]);
            for i in 0..2 {
                let mut h = siphasher::sip::SipHasher24::new_with_keys(0, i as u64);
                entity.hash(&mut h);
                cursor
                    .write_u64::<BigEndian>(h.finish())
                    .expect("write_u64 to fixed size buffer should never fail");
            }
        }
        Id {
            val,
            phantom: PhantomData,
        }
    }
}

impl<T> Distribution<Id<T>> for Standard {
    fn sample<R: ?Sized + rand::Rng>(&self, rng: &mut R) -> Id<T> {
        let val = rng.gen();
        Id {
            val,
            phantom: PhantomData,
        }
    }
}

impl<T: Entity> fmt::Display for Id<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let mut buf = [0u8; 22];
        let sz = base64::encode_config_slice(&self.val, base64::URL_SAFE_NO_PAD, &mut buf);
        assert_eq!(sz, buf.len());
        write!(
            fmt,
            "{}{}{}",
            T::PREFIX,
            DIVIDER,
            String::from_utf8_lossy(&buf)
        )?;
        Ok(())
    }
}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Id")
            .field("val", &format_args!("{:x}", self.val.as_hex()))
            .finish()
    }
}

impl<T: Entity> std::str::FromStr for Id<T> {
    type Err = Error;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        if T::PREFIX.len() > src.len() {
            bail!(IdParseError::InvalidPrefix);
        };
        let (start, remainder) = src.split_at(T::PREFIX.len());
        if start != T::PREFIX {
            bail!(IdParseError::InvalidPrefix);
        }
        if remainder.len() < 1 {
            bail!(IdParseError::Unparseable);
        }
        let (divider, b64) = remainder.split_at(1);

        if divider != DIVIDER {
            bail!(IdParseError::Unparseable);
        }

        let mut id = Id::default();
        let sz = base64::decode_config_slice(b64, base64::URL_SAFE_NO_PAD, &mut id.val)?;
        if sz != std::mem::size_of_val(&id.val) {
            bail!(IdParseError::Unparseable);
        }
        Ok(id)
    }
}

impl<T> Default for Id<T> {
    fn default() -> Self {
        let val = Default::default();
        let phantom = PhantomData;
        Id { val, phantom }
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.val == other.val
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
        self.val.cmp(&other.val)
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Id {
            val: self.val,
            phantom: self.phantom,
        }
    }
}

impl<T> Copy for Id<T> {}

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
    use rand::prelude::*;
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
        let mut rng = rand::thread_rng();

        let id = rng.gen::<Id<Canary>>();
        let id2 = rng.gen::<Id<Canary>>();

        assert_ne!(id, id2);
    }

    #[test]
    fn should_allow_ordering() {
        let mut rng = rand::thread_rng();

        let id = rng.gen::<Id<Canary>>();
        let mut id2 = rng.gen::<Id<Canary>>();
        while id2 == id {
            id2 = rng.gen::<Id<Canary>>();
        }

        assert!(id < id2 || id > id2);
    }

    #[test]
    fn to_string_should_be_prefixed_with_type_name() {
        let mut rng = rand::thread_rng();

        let id = rng.gen::<Id<Canary>>();

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
        println!("sample: {}", rand::random::<Id<Canary>>());

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
        println!("sample: {}", rand::random::<Id<Canary>>());

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
