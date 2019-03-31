use std::ops::Deref;

use failure::Error;
use postgres::Connection;
use serde::{de::DeserializeOwned, Serialize};
use serde_json;

use ids::{Entity, Id};

#[derive(Fail, Debug, PartialEq, Eq)]
#[fail(display = "stale version")]
pub struct ConcurrencyError;

pub struct Documents<C> {
    connection: C,
}

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

impl<C: Deref<Target = Connection>> Documents<C> {
    pub fn setup(&self) -> Result<(), Error> {
        self.connection.batch_execute(SETUP_SQL)?;
        Ok(())
    }

    pub fn save<D: Serialize + Entity + Versioned>(&self, document: &D) -> Result<Version, Error> {
        let json = serde_json::to_value(document)?;
        let t = self.connection.transaction()?;
        let res = if document.version() == Version::default() {
            t.prepare_cached(INSERT_SQL)?.query(&[&json])?
        } else {
            t.prepare_cached(UPDATE_SQL)?.query(&[&json])?
        };
        debug!("Query modified {} rows", res.len());
        let version = res
            .iter()
            .next()
            .ok_or_else(|| {
                warn!("Update impacted {} rows not 1", res.len());
                Error::from(ConcurrencyError)
            })?
            .get_opt(0)
            .ok_or_else(|| failure::err_msg("Missing version column?"))??;
        t.commit()?;
        Ok(Version { version })
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
impl<C> Documents<C> {
    pub fn wrap(connection: C) -> Self {
        Documents { connection }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use ids::Id;
    use r2d2::Pool;
    use r2d2_postgres::{PostgresConnectionManager, TlsMode};
    use rand::random;
    use std::env;

    const DEFAULT_URL: &'static str = "postgres://postgres@localhost/";

    #[derive(Debug)]
    struct UseTempSchema(String);

    impl r2d2::CustomizeConnection<postgres::Connection, postgres::Error> for UseTempSchema {
        fn on_acquire(&self, conn: &mut postgres::Connection) -> Result<(), postgres::Error> {
            loop {
                let t = conn.transaction()?;
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
            conn.execute(&format!("SET search_path TO \"{}\"", self.0), &[])?;
            Ok(())
        }
    }

    fn pool(schema: &str) -> Pool<PostgresConnectionManager> {
        debug!("Build pool for {}", schema);
        let url = env::var("POSTGRES_URL").unwrap_or_else(|_| DEFAULT_URL.to_string());
        debug!("Use schema name: {}", schema);
        let manager = PostgresConnectionManager::new(&*url, TlsMode::None).expect("postgres");
        let pool = r2d2::Pool::builder()
            .max_size(2)
            .connection_customizer(Box::new(UseTempSchema(schema.to_string())))
            .build(manager)
            .expect("pool");
        let conn = pool.get().expect("temp connection");
        cleanup(&conn, schema);

        debug!("Init schema in {}", schema);
        Documents::wrap(conn).setup().expect("setup");

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
        #[serde(rename = "_id")]
        id: Id<ADocument>,
        #[serde(flatten)]
        version: Version,
        name: String,
    }
    impl Entity for ADocument {
        const PREFIX: &'static str = "adocument";
    }
    impl Versioned for ADocument {
        fn version(&self) -> Version {
            self.version.clone()
        }
    }

    #[test]
    fn load_missing_document_should_return_none() {
        pretty_env_logger::try_init().unwrap_or_default();
        let pool = pool("load_missing_document_should_return_none");

        let conn = pool.get().expect("temp connection");
        let docs = Documents::wrap(&*conn);

        let loaded = docs
            .load::<ADocument>(&random::<Id<ADocument>>())
            .expect("load");
        info!("Loaded document: {:?}", loaded);

        assert_eq!(None, loaded);
    }

    #[test]
    fn save_load() {
        pretty_env_logger::try_init().unwrap_or_default();
        let pool = pool("save_load");

        let some_doc = ADocument {
            id: random(),
            name: "Dave".to_string(),
            ..Default::default()
        };

        let conn = pool.get().expect("temp connection");
        let docs = Documents::wrap(&*conn);

        info!("Original document: {:?}", some_doc);

        // Ensure we don't accidentally "find" the document by virtue of it
        // being the first in the data file.
        for _ in 0..4 {
            docs.save(&ADocument {
                id: random(),
                name: format!("{:x}", random::<usize>()),
                ..Default::default()
            })
            .expect("save");
        }
        docs.save(&some_doc).expect("save");
        for _ in 0..4 {
            docs.save(&ADocument {
                id: random(),
                name: format!("{:x}", random::<usize>()),
                ..Default::default()
            })
            .expect("save");
        }

        let loaded = docs.load(&some_doc.id).expect("load");
        info!("Loaded document: {:?}", loaded);

        assert_eq!(Some(some_doc.name), loaded.map(|d| d.name));
    }

    #[test]
    fn should_update_on_overwrite() {
        pretty_env_logger::try_init().unwrap_or_default();
        let pool = pool("should_update_on_overwrite");

        let some_doc = ADocument {
            id: random(),
            name: "Version 1".to_string(),
            ..Default::default()
        };

        let conn = pool.get().expect("temp connection");
        let docs = Documents::wrap(&*conn);

        info!("Original document: {:?}", some_doc);
        let version = docs.save(&some_doc).expect("save original");

        let modified_doc = ADocument {
            id: some_doc.id,
            name: "Version 2".to_string(),
            version: version,
        };
        info!("Modified document: {:?}", modified_doc);
        docs.save(&modified_doc).expect("save modified");

        let loaded = docs.load(&some_doc.id).expect("load");
        info!("Loaded document: {:?}", loaded);

        assert_eq!(Some(modified_doc.name), loaded.map(|d| d.name));
    }

    #[test]
    fn supports_connection() {
        pretty_env_logger::try_init().unwrap_or_default();
        let pool = pool("supports_connection");

        let some_id = random::<Id<ADocument>>();

        let conn = pool.get().expect("temp connection");
        let docs = Documents::wrap(&*conn);
        docs.save(&ADocument {
            id: some_id,
            name: "Dummy".to_string(),
            ..Default::default()
        })
        .expect("save");
        let _ = docs.load::<ADocument>(&some_id).expect("load");
    }

    #[test]
    fn should_fail_on_overwrite_with_new() {
        pretty_env_logger::try_init().unwrap_or_default();
        let pool = pool("should_fail_on_overwrite_with_new");

        let some_doc = ADocument {
            id: random(),
            name: "Version 1".to_string(),
            ..Default::default()
        };

        let conn = pool.get().expect("temp connection");
        let docs = Documents::wrap(&*conn);

        info!("Original document: {:?}", some_doc);
        docs.save(&some_doc).expect("save original");

        let modified_doc = ADocument {
            id: some_doc.id,
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
        pretty_env_logger::try_init().unwrap_or_default();
        let pool = pool("should_fail_on_overwrite_with_bogus_version");

        let some_doc = ADocument {
            id: random(),
            name: "Version 1".to_string(),
            ..Default::default()
        };

        let conn = pool.get().expect("temp connection");
        let docs = Documents::wrap(&*conn);

        info!("Original document: {:?}", some_doc);
        docs.save(&some_doc).expect("save original");

        let mut bogus_version = Version::default();
        bogus_version.version = "garbage".into();

        let modified_doc = ADocument {
            id: some_doc.id,
            name: "Version 2".to_string(),
            version: bogus_version,
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
        pretty_env_logger::try_init().unwrap_or_default();
        let pool = pool("should_fail_on_new_document_with_nonzero_version");

        let mut bogus_version = Version::default();
        bogus_version.version = "garbage".into();
        let some_doc = ADocument {
            id: random(),
            name: "Version 1".to_string(),
            version: bogus_version,
        };

        let conn = pool.get().expect("temp connection");
        let docs = Documents::wrap(&*conn);

        info!("new misversioned document: {:?}", some_doc);
        let err = docs.save(&some_doc).expect_err("save should fail");

        assert_eq!(
            err.find_root_cause().downcast_ref::<ConcurrencyError>(),
            Some(&ConcurrencyError),
            "Error: {:?}",
            err
        );
    }
}
