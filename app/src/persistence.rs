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
const INSERT_SQL: &'static str = "WITH a as (
                                SELECT $1::jsonb as body
                                )
                                INSERT INTO documents AS d (id, body)
                                SELECT a.body ->> '_id', jsonb_set(a.body, '{_version}', to_jsonb(to_hex(txid_current())))
                                FROM a
                                WHERE NOT EXISTS (
                                    SELECT 1 FROM documents d where d.id = a.body ->> '_id'
                                )
                                RETURNING d.body ->> '_version'";
const UPDATE_SQL: &'static str = "WITH a as (
                                    SELECT $1::jsonb as body
                                    )
                                    UPDATE documents AS d
                                        SET body = jsonb_set(a.body, '{_version}', to_jsonb(to_hex(txid_current())))
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
    use documents::*;
    use ids::Id;
    use r2d2::Pool;
    use r2d2_postgres::{PostgresConnectionManager, TlsMode};
    use rand::random;
    use std::env;

    const DEFAULT_URL: &'static str = "postgres://postgres@localhost/";

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

    fn pool(schema: &str) -> Pool<DocumentConnectionManager> {
        debug!("Build pool for {}", schema);
        let url = env::var("POSTGRES_URL").unwrap_or_else(|_| DEFAULT_URL.to_string());
        debug!("Use schema name: {}", schema);
        let manager = PostgresConnectionManager::new(&*url, TlsMode::None).expect("postgres");
        let pool = r2d2::Pool::builder()
            .max_size(2)
            .connection_customizer(Box::new(UseTempSchema(schema.to_string())))
            .build(DocumentConnectionManager(manager))
            .expect("pool");
        let conn = pool.get().expect("temp connection");
        cleanup(&conn.connection, schema);

        debug!("Init schema in {}", schema);
        conn.setup().expect("setup");

        pool
    }

    fn cleanup(conn: &postgres::Connection, schema: &str) {
        let t = conn.transaction().expect("begin");
        debug!("Clean old tables in {}", schema);
        for row in t
            .query(
                "SELECT n.nspname, c.relname \
                 FROM pg_catalog.pg_class c \
                 LEFT JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace \
                 WHERE n.nspname = $1 and c.relkind = 'r'",
                &[&schema],
            )
            .expect("query tables")
            .iter()
        {
            let schema = row.get::<_, String>(0);
            let table = row.get::<_, String>(1);
            t.execute(&format!("DROP TABLE {}.{}", schema, table), &[])
                .expect("drop table");
        }
        t.commit().expect("commit");
    }

    #[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, Default)]
    struct ADocument {
        #[serde(flatten)]
        meta: DocMeta<ADocument>,
        name: String,
    }
    impl Entity for ADocument {
        const PREFIX: &'static str = "adocument";
    }
    impl AsRef<DocMeta<ADocument>> for ADocument {
        fn as_ref(&self) -> &DocMeta<ADocument> {
            &self.meta
        }
    }

    #[test]
    fn load_missing_document_should_return_none() {
        env_logger::try_init().unwrap_or_default();
        let pool = pool("load_missing_document_should_return_none");

        let docs = pool.get().expect("temp connection");

        let loaded = docs
            .load::<ADocument>(&random::<Id<ADocument>>())
            .expect("load");
        info!("Loaded document: {:?}", loaded);

        assert_eq!(None, loaded);
    }

    #[test]
    fn save_load() {
        env_logger::try_init().unwrap_or_default();
        let pool = pool("save_load");

        let some_doc = ADocument {
            meta: DocMeta {
                id: random(),
                ..Default::default()
            },
            name: "Dave".to_string(),
            ..Default::default()
        };

        let docs = pool.get().expect("temp connection");

        info!("Original document: {:?}", some_doc);

        // Ensure we don't accidentally "find" the document by virtue of it
        // being the first in the data file.
        for _ in 0..4 {
            docs.save(&ADocument {
                meta: DocMeta {
                    id: random(),
                    ..Default::default()
                },
                name: format!("{:x}", random::<usize>()),
                ..Default::default()
            })
            .expect("save");
        }
        docs.save(&some_doc).expect("save");
        for _ in 0..4 {
            docs.save(&ADocument {
                meta: DocMeta {
                    id: random(),
                    ..Default::default()
                },
                name: format!("{:x}", random::<usize>()),
            })
            .expect("save");
        }

        let loaded = docs.load(&some_doc.meta.id).expect("load");
        info!("Loaded document: {:?}", loaded);

        assert_eq!(Some(some_doc.name), loaded.map(|d| d.name));
    }

    #[test]
    fn should_update_on_overwrite() {
        env_logger::try_init().unwrap_or_default();
        let pool = pool("should_update_on_overwrite");

        let some_doc = ADocument {
            meta: DocMeta {
                id: random(),
                ..Default::default()
            },
            name: "Version 1".to_string(),
            ..Default::default()
        };

        let docs = pool.get().expect("temp connection");

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
    }

    #[test]
    fn supports_connection() {
        env_logger::try_init().unwrap_or_default();
        let pool = pool("supports_connection");

        let some_id = random::<Id<ADocument>>();

        let docs = pool.get().expect("temp connection");
        docs.save(&ADocument {
            meta: DocMeta {
                id: some_id,
                ..Default::default()
            },
            name: "Dummy".to_string(),
        })
        .expect("save");
        let _ = docs.load::<ADocument>(&some_id).expect("load");
    }

    #[test]
    fn should_fail_on_overwrite_with_new() {
        env_logger::try_init().unwrap_or_default();
        let pool = pool("should_fail_on_overwrite_with_new");

        let some_doc = ADocument {
            meta: DocMeta {
                id: random(),
                ..Default::default()
            },
            name: "Version 1".to_string(),
        };

        let docs = pool.get().expect("temp connection");

        info!("Original document: {:?}", some_doc);
        docs.save(&some_doc).expect("save original");

        let modified_doc = ADocument {
            meta: DocMeta {
                version: Default::default(),
                ..some_doc.meta
            },
            name: "Version 2".to_string(),
            ..Default::default()
        };

        info!("Modified document: {:?}", modified_doc);
        let err = docs.save(&modified_doc).expect_err("save should fail");

        assert_eq!(
            err.find_root_cause().downcast_ref::<ConcurrencyError>(),
            Some(&ConcurrencyError),
            "Error: {:?}",
            err
        );
    }

    #[test]
    fn should_fail_on_overwrite_with_bogus_version() {
        env_logger::try_init().unwrap_or_default();
        let pool = pool("should_fail_on_overwrite_with_bogus_version");

        let some_doc = ADocument {
            meta: DocMeta {
                id: random(),
                ..Default::default()
            },
            name: "Version 1".to_string(),
            ..Default::default()
        };

        let docs = pool.get().expect("temp connection");

        info!("Original document: {:?}", some_doc);
        docs.save(&some_doc).expect("save original");

        let modified_doc = ADocument {
            meta: DocMeta {
                id: some_doc.meta.id,
                version: Version::from_str("garbage").expect("garbage version"),
                ..Default::default()
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
    }

    #[test]
    fn should_fail_on_new_document_with_nonzero_version() {
        env_logger::try_init().unwrap_or_default();
        let pool = pool("should_fail_on_new_document_with_nonzero_version");

        let some_doc = ADocument {
            meta: DocMeta {
                id: random(),
                version: Version::from_str("garbage").expect("garbage version"),
                ..Default::default()
            },
            name: "Version 1".to_string(),
        };

        let docs = pool.get().expect("temp connection");

        info!("new misAsRef<DocMeta> document: {:?}", some_doc);
        let err = docs.save(&some_doc).expect_err("save should fail");

        assert_eq!(
            err.find_root_cause().downcast_ref::<ConcurrencyError>(),
            Some(&ConcurrencyError),
            "Error: {:?}",
            err
        );
    }
}
