use std::str::FromStr;

use failure::Error;
use serde::{de::DeserializeOwned, Serialize};
use serde_json;

use r2d2_postgres::PostgresConnectionManager;

use documents::{DocMeta, Version};
use ids::{Entity, Id};

#[derive(Fail, Debug, PartialEq, Eq)]
#[fail(display = "stale version")]
pub struct ConcurrencyError;

pub struct Documents {
    connection: postgres::Connection,
}

#[derive(Debug)]
pub struct DocumentConnectionManager(PostgresConnectionManager);

const SETUP_SQL: &'static str = include_str!("persistence.sql");
const LOAD_SQL: &'static str = "SELECT body FROM documents WHERE id = $1";
#[cfg(test)]
const LOAD_NEXT_SQL: &'static str = "SELECT body
                                     FROM documents
                                     WHERE jsonb_array_length(body -> '_outgoing') > 0
                                     LIMIT 1
";
const INSERT_SQL: &'static str = "WITH a as (
                                SELECT $1::jsonb as body
                                )
                                INSERT INTO documents AS d (id, body)
                                SELECT a.body ->> '_id',
                                    a.body || jsonb_build_object('_version', to_hex(txid_current()))
                                FROM a
                                WHERE NOT EXISTS (
                                    SELECT 1 FROM documents d where d.id = a.body ->> '_id'
                                )
                                RETURNING d.body ->> '_version'";
const UPDATE_SQL: &'static str = "WITH a as (
                                    SELECT $1::jsonb as body
                                    )
                                    UPDATE documents AS d
                                        SET body = a.body
                                            || jsonb_build_object('_version', to_hex(txid_current()))
                                        FROM a
                                        WHERE id = a.body ->> '_id'
                                        AND d.body -> '_version' = a.body -> '_version'
                                        RETURNING d.body ->> '_version'
                                    ";

impl Documents {
    pub fn setup(&self) -> Result<(), Error> {
        for stmt in SETUP_SQL.split("\n\n") {
            self.connection.batch_execute(stmt)?;
        }
        Ok(())
    }

    pub fn save<D: Serialize + Entity + AsRef<DocMeta<D>>>(
        &self,
        document: &D,
    ) -> Result<Version, Error> {
        let json = serde_json::to_value(document)?;
        let t = self.connection.transaction()?;
        let res = if document.as_ref().version == Version::default() {
            t.prepare_cached(INSERT_SQL)?.query(&[&json])?
        } else {
            t.prepare_cached(UPDATE_SQL)?.query(&[&json])?
        };
        debug!("Query modified {} rows", res.len());
        let version: String = res
            .iter()
            .next()
            .ok_or_else(|| {
                warn!("Update impacted {} rows not 1", res.len());
                Error::from(ConcurrencyError)
            })?
            .get_opt(0)
            .ok_or_else(|| failure::err_msg("Missing version column?"))??;
        t.commit()?;
        Ok(Version::from_str(&version)?)
    }

    pub fn load<D: DeserializeOwned + Entity>(&self, id: &Id<D>) -> Result<Option<D>, Error> {
        let load = self.connection.prepare_cached(LOAD_SQL)?;
        let res = load.query(&[&id.to_string()])?;

        if let Some(row) = res.iter().next() {
            let json: serde_json::Value = row.get_opt(0).expect("Missing column in row?")?;
            let doc = serde_json::from_value(json)?;

            Ok(Some(doc))
        } else {
            Ok(None)
        }
    }

    #[cfg(test)]
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

impl DocumentConnectionManager {
    pub fn new(pg: PostgresConnectionManager) -> Self {
        DocumentConnectionManager(pg)
    }
}
impl r2d2::ManageConnection for DocumentConnectionManager {
    type Connection = Documents;
    type Error = postgres::Error;

    fn connect(&self) -> Result<Self::Connection, Self::Error> {
        let connection = self.0.connect()?;
        Ok(Documents { connection })
    }

    fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        Ok(PostgresConnectionManager::is_valid(
            &self.0,
            &mut conn.connection,
        )?)
    }

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        PostgresConnectionManager::has_broken(&self.0, &mut conn.connection)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ids;
    use documents::*;
    use failure::ResultExt;
    use r2d2::Pool;
    use r2d2_postgres::{PostgresConnectionManager, TlsMode};
    use rand::random;
    use std::env;

    lazy_static! {
        static ref IDGEN: ids::IdGen = ids::IdGen::new();
    }

    #[derive(Debug)]
    struct UseTempSchema(String);

    impl r2d2::CustomizeConnection<Documents, postgres::Error> for UseTempSchema {
        fn on_acquire(&self, conn: &mut Documents) -> Result<(), postgres::Error> {
            loop {
                let t = conn.connection.transaction()?;
                let nschemas: i64 = {
                    let rows = t.query(
                        "SELECT count(*) from pg_catalog.pg_namespace n where n.nspname = $1",
                        &[&self.0],
                    )?;
                    let row = rows.get(0);
                    row.get(0)
                };
                debug!("Number of {} schemas:{}", self.0, nschemas);
                if nschemas == 0 {
                    match t.execute(&format!("CREATE SCHEMA \"{}\"", self.0), &[]) {
                        Ok(_) => {
                            t.commit()?;
                            break;
                        }
                        Err(e) => warn!("Error creating schema:{:?}: {:?}", self.0, e),
                    }
                } else {
                    break;
                }
            }
            conn.connection
                .execute(&format!("SET search_path TO \"{}\"", self.0), &[])?;
            Ok(())
        }
    }

    fn pool(schema: &str) -> Result<Pool<DocumentConnectionManager>, Error> {
        debug!("Build pool for {}", schema);
        let url = env::var("POSTGRES_URL").context("$POSTGRES_URL")?;
        debug!("Use schema name: {}", schema);
        let manager = PostgresConnectionManager::new(&*url, TlsMode::None).expect("postgres");
        let pool = r2d2::Pool::builder()
            .max_size(2)
            .connection_customizer(Box::new(UseTempSchema(schema.to_string())))
            .build(DocumentConnectionManager(manager))?;

        let conn = pool.get()?;
        cleanup(&conn.connection, schema)?;

        debug!("Init schema in {}", schema);
        conn.setup()?;

        Ok(pool)
    }

    fn cleanup(conn: &postgres::Connection, schema: &str) -> Result<(), Error> {
        let t = conn.transaction()?;
        debug!("Clean old tables in {}", schema);
        for row in t
            .query(
                "SELECT n.nspname, c.relname \
                 FROM pg_catalog.pg_class c \
                 LEFT JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace \
                 WHERE n.nspname = $1 and c.relkind = 'r'",
                &[&schema],
            )?
            .iter()
        {
            let schema = row.get::<_, String>(0);
            let table = row.get::<_, String>(1);
            t.execute(&format!("DROP TABLE {}.{}", schema, table), &[])?;
        }
        t.commit()?;
        Ok(())
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
            docs.save(&ADocument {
                meta: DocMeta::new_with_id(IDGEN.generate()),
                name: format!("{:x}", random::<usize>()),
            })
            .expect("save");
        }
        docs.save(&some_doc).expect("save");
        for _ in 0..4 {
            docs.save(&ADocument {
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
        let version = docs.save(&some_doc).expect("save original");

        let modified_doc = ADocument {
            meta: DocMeta {
                version: version,
                ..some_doc.meta
            },
            name: "Version 2".to_string(),
        };
        info!("Modified document: {:?}", modified_doc);
        docs.save(&modified_doc).expect("save modified");

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
        docs.save(&ADocument {
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
        docs.save(&some_doc).expect("save original");

        let modified_doc = ADocument {
            meta: DocMeta {
                version: Default::default(),
                ..some_doc.meta
            },
            name: "Version 2".to_string(),
        };

        info!("Modified document: {:?}", modified_doc);
        let err = docs.save(&modified_doc).expect_err("save should fail");

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
        let actual = docs.save(&some_doc).expect("save original");

        let modified_doc = ADocument {
            meta: DocMeta::new_with_id(id),
            name: "Version 2".to_string(),
        };

        assert_ne!(actual, modified_doc.meta.version);

        info!("Modified document: {:?}", modified_doc);
        let err = docs.save(&modified_doc).expect_err("save should fail");

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
        meta.version = Version::from_str("garbage").expect("garbage version");
        let name = "Version 1".to_string();
        let some_doc = ADocument { meta, name };

        let docs = pool.get()?;

        info!("new misAsRef<DocMeta> document: {:?}", some_doc);
        let err = docs.save(&some_doc).expect_err("save should fail");

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
    fn should_enqueue_on_create() -> Result<(), Error> {
        env_logger::try_init().unwrap_or_default();
        let pool = pool("should_enqueue_on_create")?;
        let docs = pool.get()?;

        let mut some_doc = ChattyDoc {
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
    fn should_enqueue_on_update() -> Result<(), Error> {
        env_logger::try_init().unwrap_or_default();
        let pool = pool("should_enqueue_on_update")?;
        let docs = pool.get()?;

        let mut some_doc = ChattyDoc {
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
    #[ignore]
    fn should_enqueue_something_something() -> Result<(), Error> {
        env_logger::try_init().unwrap_or_default();
        let pool = pool("should_enqueue_something_something")?;

        let mut some_doc = ChattyDoc {
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
