use std::convert::TryInto;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::time::{Duration, SystemTime};

use anyhow::Error;
use data_encoding::BASE32_DNSSEC;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

use crate::ids::{Id, IdGen, IdParseError, ENCODED_BARE_ID_LEN};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct UntypedId {
    // Unix time in ms
    pub(crate) stamp: u64,
    pub(crate) random: u64,
}

impl IdGen {
    pub fn untyped(&self) -> UntypedId {
        let stamp_epoch = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("now");
        let stamp_s: u64 = stamp_epoch
            .as_secs()
            .checked_mul(1000 * 1000 * 1000)
            .expect("secs * 1000,000,000");
        let stamp_ms: u64 = stamp_epoch.subsec_nanos().into();
        let stamp = stamp_s + stamp_ms;
        let random = rand::random();

        UntypedId { random, stamp }
    }
}

impl UntypedId {
    pub(crate) fn from_bytes(bytes: &[u8]) -> Self {
        let stamp = u64::from_be_bytes(bytes[0..8].try_into().expect("stamp bytes"));
        let random = u64::from_be_bytes(bytes[8..8 + 8].try_into().expect("random bytes"));

        UntypedId { stamp, random }
    }

    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(16);
        bytes.extend(&self.stamp.to_be_bytes());
        bytes.extend(&self.random.to_be_bytes());
        bytes
    }

    /// Returns a id nominally at time zero, but with a random portion derived
    /// from the given entity.
    pub fn hashed<H: Hash>(entity: H) -> Self {
        let stamp_limit_ns = (1 << 30) * 1_000_000_000;
        let raw_stamp = sip_hash(0, 1, &entity);
        // Rescale the value from 0..u64::max_value() to 0..stamp_limit_ns;
        let stamp = ((u128::from(raw_stamp) * stamp_limit_ns) >> 64)
            .try_into()
            .unwrap();
        let random = sip_hash(0, 0, &entity);

        UntypedId { stamp, random }
    }

    pub fn typed<T>(&self) -> Id<T> {
        Id::from_untyped(*self)
    }

    pub fn timestamp(&self) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_nanos(self.stamp)
    }

    pub fn random(&self) -> u64 {
        self.random
    }
}

fn sip_hash<H: Hash>(k0: u64, k1: u64, entity: &H) -> u64 {
    let mut h = siphasher::sip::SipHasher24::new_with_keys(k0, k1);
    entity.hash(&mut h);
    h.finish()
}

impl std::str::FromStr for UntypedId {
    type Err = Error;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let mut bytes = [0u8; 16];
        if src.len() != ENCODED_BARE_ID_LEN {
            return Err(IdParseError::Unparseable.into());
        }
        BASE32_DNSSEC
            .decode_mut(src.as_bytes(), &mut bytes)
            .map_err(IdParseError::from)?;

        Ok(Self::from_bytes(&bytes[..]))
    }
}

impl fmt::Display for UntypedId {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = [0u8; ENCODED_BARE_ID_LEN];
        BASE32_DNSSEC.encode_mut(&self.to_bytes(), &mut buf);

        write!(fmt, "{}", String::from_utf8_lossy(&buf[..]))?;
        Ok(())
    }
}

impl Serialize for UntypedId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for UntypedId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct IdStrVisitor;
        impl<'vi> de::Visitor<'vi> for IdStrVisitor {
            type Value = UntypedId;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "an UntypedId string")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<UntypedId, E> {
                value.parse::<UntypedId>().map_err(E::custom)
            }
        }

        deserializer.deserialize_str(IdStrVisitor)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json;

    #[test]
    fn round_trips_via_to_from_str() {
        let id = UntypedId::hashed(&"Hi!");
        let s = id.to_string();
        println!("String: {}", s);
        let id2 = s.parse::<UntypedId>().expect("parse id");
        assert_eq!(id, id2);
    }

    #[test]
    fn round_trips_via_to_from_str_now() {
        let id = IdGen::new().untyped();
        let s = id.to_string();
        println!("String: {}", s);
        let id2 = s.parse::<UntypedId>().expect("parse id");
        assert_eq!(id, id2);
    }

    #[test]
    fn round_trips_via_serde_json() {
        let id = UntypedId::hashed(&"boo");

        let json = serde_json::to_string(&id).expect("serde_json::to_string");
        println!("Json: {}", json);
        let id2 = serde_json::from_str(&json).expect("serde_json::from_str");
        assert_eq!(id, id2);
    }

    #[test]
    fn serializes_to_string_like() {
        let id = UntypedId::hashed(&"boo");

        let json = serde_json::to_string(&id).expect("serde_json::to_string");
        let s: String = serde_json::from_str(&json).expect("serde_json::from_str");
        assert_eq!(id.to_string(), s);
    }

    #[test]
    fn should_allow_random_generation() {
        let idgen = IdGen::new();
        let id = idgen.untyped();
        let id2 = idgen.untyped();

        assert_ne!(id, id2);
    }

    #[test]
    fn should_allow_ordering() {
        let idgen = IdGen::new();
        let id = idgen.untyped();
        let mut id2 = idgen.untyped();
        while id2 == id {
            id2 = idgen.untyped();
        }

        assert!(id < id2 || id > id2);
    }

    #[test]
    fn should_parse_expected_len() {
        let s = "0000000000001q5nnvfqq7krfo";

        let result = s.parse::<UntypedId>();

        assert!(
            result.is_ok(),
            "Parsing {:?} should return ok; got {:?}",
            s,
            result,
        )
    }

    #[test]
    fn should_verify_has_no_entity_prefix() {
        let s = "wrong.0000000000001q5nnvfqq7krfo";

        let result = s.parse::<UntypedId>();

        assert!(
            result.is_err(),
            "Parsing {:?} should return error; got {:?}",
            s,
            result,
        )
    }

    #[test]
    fn should_return_error_on_truncation() {
        let s = "0000000000001q5nnvfqq7krf";

        let result = s.parse::<UntypedId>();

        assert!(
            result.is_err(),
            "Parsing {:?} should return error; got {:?}",
            s,
            result,
        )
    }
    #[test]
    fn should_return_error_on_extension() {
        let s = "0000000000001q5nnvfqq7krfoa";

        let result = s.parse::<UntypedId>();

        assert!(
            result.is_err(),
            "Parsing {:?} should return error; got {:?}",
            s,
            result,
        )
    }

    #[test]
    fn hashed_generates_bounded_timestamps() {
        // Show that we can generate timestamps that are before an "impossible"
        // time.
        // We can safely assume that we won't be generating timestamps before
        // the unix timestamp for 2^30, or about 2004-01-10T13:37:04Z.
        for x in 0u64..4096 {
            let id = UntypedId::hashed(&x);
            println!("in:{}; id:{}", x, id);
            let unix_s = id
                .timestamp()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("since epoch")
                .as_secs();
            let log2_s = (unix_s as f64).log2();
            let limit = 30.0;
            assert!(
                log2_s < limit,
                "log2(Seconds since unix epoch) for {:?} should be < {};\
                 for input:{}; got:{}",
                id.timestamp(),
                limit,
                x,
                log2_s,
            );
        }
    }
}
