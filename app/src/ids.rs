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

impl<T> fmt::Display for Id<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let mut buf = [0u8; 22];
        let sz = base64::encode_config_slice(&self.val, base64::URL_SAFE_NO_PAD, &mut buf);
        assert_eq!(sz, buf.len());
        write!(fmt, "{}", String::from_utf8_lossy(&buf))?;
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

impl<T> std::str::FromStr for Id<T> {
    type Err = Error;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let mut id = Id::default();
        let sz = base64::decode_config_slice(src, base64::URL_SAFE_NO_PAD, &mut id.val)?;
        if sz != std::mem::size_of_val(&id.val) {
            bail!("Could not decode id from base64: {:?}", src)
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

impl<T> Serialize for Id<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de, T> Deserialize<'de> for Id<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct IdStrVisitor<T>(PhantomData<T>);
        impl<'vi, T> de::Visitor<'vi> for IdStrVisitor<T> {
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

}
