use byteorder::{BigEndian, WriteBytesExt};
use failure::Error;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::io;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Default)]
pub struct Id {
    val: [u8; 16],
}

impl Id {
    pub fn of<H: Hash>(entity: &H) -> Self {
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
        Id { val }
    }
}

impl fmt::Display for Id {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let mut buf = [0u8; 22];
        let sz = base64::encode_config_slice(&self.val, base64::URL_SAFE_NO_PAD, &mut buf);
        assert_eq!(sz, buf.len());
        write!(fmt, "{}", String::from_utf8_lossy(&buf))?;
        Ok(())
    }
}

impl std::str::FromStr for Id {
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
