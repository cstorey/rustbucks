use failure::Error;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default, Hash)]
pub struct Version {
    #[serde(rename = "_version")]
    version: String,
}

// This is quite nasty; as at present we assume that the version is both _here_ as well as
// in the `_version` property.
pub trait Versioned {
    fn version(&self) -> Version;
}

impl std::str::FromStr for Version {
    type Err = Error;
    fn from_str(val: &str) -> Result<Self, Error> {
        let version = val.to_string();
        Ok(Version { version })
    }
}
