use std::fmt;
use std::sync::Arc;

use failure::{Error, Fail};
use log::*;
use serde::{de::DeserializeOwned, Serialize};
use serde_json;

use crate::documents::{DocMeta, Version};
use crate::ids::{Entity, Id};

pub trait Storage {
    fn load<D: DeserializeOwned + Entity>(&self, id: &Id<D>) -> Result<Option<D>, Error>;
    fn save<D: DeserializeOwned + Serialize + Entity + AsMut<DocMeta<D>> + AsRef<DocMeta<D>>>(
        &self,
        document: &mut D,
    ) -> Result<Version, Error>;
}

#[derive(Fail, Debug, PartialEq, Eq)]
#[fail(display = "stale version")]
pub struct ConcurrencyError;

pub struct Documents {
    db: Arc<sled::Tree>,
}

pub struct DocumentConnectionManager(sled::Db);

impl fmt::Debug for DocumentConnectionManager {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_tuple("DocumentConnectionManager").finish()
    }
}
impl Documents {
    pub fn setup(&self) -> Result<(), Error> {
        Ok(())
    }

    pub fn save<
        D: DeserializeOwned + Serialize + Entity + AsMut<DocMeta<D>> + AsRef<DocMeta<D>>,
    >(
        &self,
        document: &mut D,
    ) -> Result<Version, Error> {
        let mut current = self.db.get(document.as_ref().id.to_string())?;
        {
            let trim_len = 80;
            trace!(
                "Loaded current version: {:?}",
                current.as_ref().map(|current| {
                    let mut out = String::new();
                    out.push_str(&String::from_utf8_lossy(
                        &current[..std::cmp::min(current.len(), trim_len)],
                    ));
                    if current.len() > trim_len {
                        out.push_str("…")
                    };
                    out
                })
            );
        }
        loop {
            let current_doc: Option<D> = if let Some(raw) = current.as_ref() {
                Some(serde_json::from_slice(&raw)?)
            } else {
                None
            };

            let current_version = current_doc
                .as_ref()
                .map(|d| d.as_ref().version.clone())
                .unwrap_or_default();
            debug!(
                "Current version: {:?}; supplied version: {:?}",
                current_version,
                document.as_ref().version
            );
            if current_version != document.as_ref().version {
                debug!(
                    "Bailing because of version mismatch ({:?} != {:?})",
                    current_version,
                    document.as_ref().version
                );
                return Err(ConcurrencyError.into());
            }

            document.as_mut().version.next();

            let json = serde_json::to_vec(document)?;

            match self
                .db
                .cas(&document.as_mut().id.to_string(), current, Some(json))?
            {
                Ok(()) => {
                    debug!("Saved okay at {:?}", document.as_ref().version);
                    return Ok(document.as_ref().version.clone());
                }
                Err(val) => {
                    debug!("Retrying");
                    current = val;
                }
            }
        }
    }

    pub fn load<D: DeserializeOwned + Entity>(&self, id: &Id<D>) -> Result<Option<D>, Error> {
        if let Some(raw) = self.db.get(id.to_string())? {
            let doc = serde_json::from_slice(&raw)?;
            Ok(Some(doc))
        } else {
            Ok(None)
        }
    }

    #[cfg(test)]
    #[cfg(never)]
    pub fn load_next_unsent<D: DeserializeOwned + Entity>(&self) -> Result<Option<D>, Error> {
        let load = self.connection.prepare_cached(LOAD_NEXT_SQL)?;
        let res = load.query(&[])?;
        debug!("Cols: {:?}; Rows: {:?}", res.columns(), res.len());

        if let Some(row) = res.iter().next() {
            let json: serde_json::Value = row.get_opt(0).expect("Missing column in row?")?;
            let doc = serde_json::from_value(json)?;

            Ok(Some(doc))
        } else {
            Ok(None)
        }
    }
}

impl Storage for Documents {
    fn load<D: DeserializeOwned + Entity>(&self, id: &Id<D>) -> Result<Option<D>, Error> {
        Documents::load(self, id)
    }

    fn save<D: DeserializeOwned + Serialize + Entity + AsMut<DocMeta<D>> + AsRef<DocMeta<D>>>(
        &self,
        document: &mut D,
    ) -> Result<Version, Error> {
        Documents::save(self, document)
    }
}

impl DocumentConnectionManager {
    pub fn new(pg: sled::Db) -> Self {
        DocumentConnectionManager(pg)
    }
}
impl r2d2::ManageConnection for DocumentConnectionManager {
    type Connection = Documents;
    type Error = sled::Error;

    fn connect(&self) -> Result<Self::Connection, Self::Error> {
        debug!("Open tree");
        Ok(Documents {
            db: self.0.open_tree(b"documents")?,
        })
    }

    fn is_valid(&self, _conn: &mut Self::Connection) -> Result<(), Self::Error> {
        Ok(())
    }

    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        false
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::documents::*;
    use crate::ids;
    use lazy_static::lazy_static;
    use r2d2::Pool;
    use rand::random;
    use serde::{Deserialize, Serialize};
    use sled;

    lazy_static! {
        static ref IDGEN: ids::IdGen = ids::IdGen::new();
    }

    fn pool(schema: &str) -> Result<Pool<DocumentConnectionManager>, Error> {
        debug!("Build pool for {}", schema);
        let path = tempfile::TempDir::new()?.into_path();
        let db = sled::Db::start_default(path)?;
        debug!("Use schema name: {}", schema);
        let pool = r2d2::Pool::builder()
            .max_size(2)
            .build(DocumentConnectionManager::new(db))?;

        let conn = pool.get()?;

        debug!("Init schema in {}", schema);
        conn.setup()?;

        Ok(pool)
    }

    #[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
    struct ADocument {
        #[serde(flatten)]
        meta: DocMeta<ADocument>,
        name: String,
    }

    #[derive(Debug, Clone, Default, Hash, PartialEq, Eq, Deserialize, Serialize)]
    struct AMessage;
    impl Entity for ADocument {
        const PREFIX: &'static str = "adocument";
    }
    impl AsRef<DocMeta<ADocument>> for ADocument {
        fn as_ref(&self) -> &DocMeta<Self> {
            &self.meta
        }
    }

    impl AsMut<DocMeta<ADocument>> for ADocument {
        fn as_mut(&mut self) -> &mut DocMeta<Self> {
            &mut self.meta
        }
    }

    #[test]
    fn load_missing_document_should_return_none() -> Result<(), Error> {
        env_logger::try_init().unwrap_or_default();
        let pool = pool("load_missing_document_should_return_none")?;

        let docs = pool.get()?;

        let loaded = docs.load::<ADocument>(&IDGEN.generate()).expect("load");
        info!("Loaded document: {:?}", loaded);

        assert_eq!(None, loaded);
        Ok(())
    }

    #[test]
    fn save_load() -> Result<(), Error> {
        env_logger::try_init().unwrap_or_default();
        let pool = pool("save_load")?;
        let some_doc = ADocument {
            meta: DocMeta::new_with_id(IDGEN.generate()),
            name: "Dave".to_string(),
        };

        let docs = pool.get()?;

        info!("Original document: {:?}", some_doc);

        // Ensure we don't accidentally "find" the document by virtue of it
        // being the first in the data file.
        for _ in 0..4 {
            docs.save(&mut ADocument {
                meta: DocMeta::new_with_id(IDGEN.generate()),
                name: format!("{:x}", random::<usize>()),
            })
            .expect("save");
        }
        docs.save(&mut some_doc.clone()).expect("save");
        for _ in 0..4 {
            docs.save(&mut ADocument {
                meta: DocMeta::new_with_id(IDGEN.generate()),
                name: format!("{:x}", random::<usize>()),
            })
            .expect("save");
        }

        let loaded = docs.load(&some_doc.meta.id).expect("load");
        info!("Loaded document: {:?}", loaded);

        assert_eq!(Some(some_doc.name), loaded.map(|d| d.name));
        Ok(())
    }

    #[test]
    fn should_update_on_overwrite() -> Result<(), Error> {
        env_logger::try_init().unwrap_or_default();
        let pool = pool("should_update_on_overwrite")?;

        let some_doc = ADocument {
            meta: DocMeta::new_with_id(IDGEN.generate()),
            name: "Version 1".to_string(),
        };

        let docs = pool.get()?;

        info!("Original document: {:?}", some_doc);
        let version = docs.save(&mut some_doc.clone()).expect("save original");

        let modified_doc = ADocument {
            meta: DocMeta {
                version: version,
                ..some_doc.meta
            },
            name: "Version 2".to_string(),
        };
        info!("Modified document: {:?}", modified_doc);
        docs.save(&mut modified_doc.clone()).expect("save modified");

        let loaded = docs.load(&some_doc.meta.id).expect("load");
        info!("Loaded document: {:?}", loaded);

        assert_eq!(Some(modified_doc.name), loaded.map(|d| d.name));
        Ok(())
    }

    #[test]
    fn supports_connection() -> Result<(), Error> {
        env_logger::try_init().unwrap_or_default();
        let pool = pool("supports_connection")?;

        let some_id = IDGEN.generate();

        let docs = pool.get()?;
        docs.save(&mut ADocument {
            meta: DocMeta::new_with_id(IDGEN.generate()),
            name: "Dummy".to_string(),
        })
        .expect("save");
        let _ = docs.load::<ADocument>(&some_id).expect("load");
        Ok(())
    }

    #[test]
    fn should_fail_on_overwrite_with_new() -> Result<(), Error> {
        env_logger::try_init().unwrap_or_default();
        let pool = pool("should_fail_on_overwrite_with_new")?;

        let some_doc = ADocument {
            meta: DocMeta::new_with_id(IDGEN.generate()),
            name: "Version 1".to_string(),
        };

        let docs = pool.get()?;

        info!("Original document: {:?}", some_doc);
        docs.save(&mut some_doc.clone()).expect("save original");

        let modified_doc = ADocument {
            meta: DocMeta {
                version: Default::default(),
                ..some_doc.meta
            },
            name: "Version 2".to_string(),
        };

        info!("Modified document: {:?}", modified_doc);
        let err = docs
            .save(&mut modified_doc.clone())
            .expect_err("save should fail");

        info!("Save failed with: {:?}", err);
        info!("root cause: {:?}", err.find_root_cause());
        assert_eq!(
            err.find_root_cause().downcast_ref::<ConcurrencyError>(),
            Some(&ConcurrencyError),
            "Error: {:?}",
            err
        );
        Ok(())
    }

    #[test]
    fn should_fail_on_overwrite_with_bogus_version() -> Result<(), Error> {
        env_logger::try_init().unwrap_or_default();
        let pool = pool("should_fail_on_overwrite_with_bogus_version")?;

        let id = IDGEN.generate();
        let some_doc = ADocument {
            meta: DocMeta::new_with_id(id),
            name: "Version 1".to_string(),
        };

        let docs = pool.get()?;

        info!("Original document: {:?}", some_doc);
        let actual = docs.save(&mut some_doc.clone()).expect("save original");

        let modified_doc = ADocument {
            meta: DocMeta::new_with_id(id),
            name: "Version 2".to_string(),
        };

        assert_ne!(actual, modified_doc.meta.version);

        info!("Modified document: {:?}", modified_doc);
        let err = docs
            .save(&mut modified_doc.clone())
            .expect_err("save should fail");

        assert_eq!(
            err.find_root_cause().downcast_ref::<ConcurrencyError>(),
            Some(&ConcurrencyError),
            "Error: {:?}",
            err
        );
        Ok(())
    }

    #[test]
    fn should_fail_on_new_document_with_nonzero_version() -> Result<(), Error> {
        env_logger::try_init().unwrap_or_default();
        let pool = pool("should_fail_on_new_document_with_nonzero_version")?;

        let mut meta = DocMeta::new_with_id(IDGEN.generate());
        meta.version = Version::from_u64(42);
        let name = "Version 1".to_string();
        let some_doc = ADocument { meta, name };

        let docs = pool.get()?;

        info!("new misAsRef<DocMeta> document: {:?}", some_doc);
        let err = docs
            .save(&mut some_doc.clone())
            .expect_err("save should fail");

        assert_eq!(
            err.find_root_cause().downcast_ref::<ConcurrencyError>(),
            Some(&ConcurrencyError),
            "Error: {:?}",
            err
        );
        Ok(())
    }

    #[derive(Clone, Debug, Deserialize, Serialize)]
    struct ChattyDoc {
        #[serde(flatten)]
        meta: DocMeta<ChattyDoc>,
        #[serde(flatten)]
        mbox: MailBox<AMessage>,
    }

    impl Entity for ChattyDoc {
        const PREFIX: &'static str = "chatty";
    }
    impl AsRef<DocMeta<ChattyDoc>> for ChattyDoc {
        fn as_ref(&self) -> &DocMeta<Self> {
            &self.meta
        }
    }

    #[test]
    #[cfg(never)]

    fn should_enqueue_nothing_by_default() -> Result<(), Error> {
        env_logger::try_init().unwrap_or_default();
        let pool = pool("should_enqueue_nothing_by_default")?;
        let docs = pool.get()?;

        let some_doc = ChattyDoc {
            meta: DocMeta::new_with_id(IDGEN.generate()),
            mbox: MailBox::default(),
        };

        info!("Original document: {:?}", some_doc);

        docs.save(&some_doc).expect("save");

        let docp = docs.load_next_unsent::<ChattyDoc>()?;
        info!("Loaded something: {:?}", docp);

        assert!(docp.is_none(), "Should find no document. Got: {:?}", docp);
        Ok(())
    }

    #[test]
    #[cfg(never)]

    fn should_enqueue_on_create() -> Result<(), Error> {
        env_logger::try_init().unwrap_or_default();
        let pool = pool("should_enqueue_on_create")?;
        let docs = pool.get()?;

        let some_doc = ChattyDoc {
            meta: DocMeta::new_with_id(IDGEN.generate()),
            mbox: MailBox::default(),
        };

        some_doc.mbox.send(AMessage);
        info!("Original document: {:?}", some_doc);
        docs.save(&some_doc).expect("save");

        let docp = docs.load_next_unsent::<ChattyDoc>()?;
        info!("Loaded something: {:?}", docp);

        let loaded = docs.load_next_unsent::<ChattyDoc>()?;
        info!("Loaded something: {:?}", loaded);

        assert_eq!(Some(some_doc.meta.id), loaded.map(|d| d.meta.id));

        Ok(())
    }

    #[test]
    #[cfg(never)]

    fn should_enqueue_on_update() -> Result<(), Error> {
        env_logger::try_init().unwrap_or_default();
        let pool = pool("should_enqueue_on_update")?;
        let docs = pool.get()?;

        let some_doc = ChattyDoc {
            meta: DocMeta::new_with_id(IDGEN.generate()),
            mbox: MailBox::default(),
        };

        let vers = docs.save(&some_doc)?;
        some_doc.meta.version = vers;

        some_doc.mbox.send(AMessage);
        info!("Original document: {:?}", some_doc);
        docs.save(&some_doc).expect("save");

        let loaded = docs.load_next_unsent::<ChattyDoc>()?;
        info!("Loaded something: {:?}", loaded);

        assert_eq!(Some(some_doc.meta.id), loaded.map(|d| d.meta.id));
        Ok(())
    }

    #[test]
    #[cfg(never)]
    #[ignore]
    fn should_enqueue_something_something() -> Result<(), Error> {
        env_logger::try_init().unwrap_or_default();
        let pool = pool("should_enqueue_something_something")?;

        let some_doc = ChattyDoc {
            meta: DocMeta::new_with_id(IDGEN.generate()),
            mbox: MailBox::default(),
        };
        some_doc.mbox.send(AMessage);

        let docs = pool.get()?;
        info!("Original document: {:?}", some_doc);

        let vers = docs.save(&some_doc)?;
        some_doc.meta.version = vers;

        let doc = docs
            .load_next_unsent::<ChattyDoc>()?
            .ok_or_else(|| failure::err_msg("missing document?"))?;;
        info!("Loaded something: {:?}", doc);

        assert_eq!(doc.meta.id, some_doc.meta.id);

        Ok(())
    }

    #[test]
    #[ignore]
    fn should_only_load_messages_of_type() {}
}
