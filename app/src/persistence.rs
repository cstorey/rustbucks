use failure::Error;
use postgres::GenericConnection;
use serde::{de::DeserializeOwned, Serialize};
use serde_json;

use ids::{Entity, Id};

pub struct Documents<'a> {
    connection: &'a GenericConnection,
}

const SETUP_SQL: &'static str = include_str!("persistence.sql");
const SAVE_SQL: &'static str = "INSERT INTO documents (id, body) VALUES ($1, $2) \
                                ON CONFLICT (id) DO UPDATE set body = EXCLUDED.body";
const LOAD_SQL: &'static str = "SELECT body FROM documents WHERE id = $1";

impl<'a> Documents<'a> {
    pub fn setup(&self) -> Result<(), Error> {
        self.connection.execute(SETUP_SQL, &[])?;
        Ok(())
    }

    pub fn wrap(connection: &'a GenericConnection) -> Self {
        Documents { connection }
    }

    pub fn save<D: Serialize + Entity>(&self, id: &Id<D>, document: &D) -> Result<(), Error> {
        let json = serde_json::to_value(document)?;
        let save = self.connection.prepare_cached(SAVE_SQL)?;
        save.execute(&[&id.to_string(), &json])?;
        Ok(())
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

        debug!("Init schema in {}", schema);
        Documents::wrap(&t).setup().expect("setup");
        t.commit().expect("commit");

        pool
    }

    #[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
    struct ADocument {
        gubbins: u64,
    }
    impl Entity for ADocument {
        const PREFIX: &'static str = "adocument";
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

        let some_id = random::<Id<ADocument>>();
        let some_doc = ADocument { gubbins: random() };

        let conn = pool.get().expect("temp connection");
        let docs = Documents::wrap(&*conn);

        info!("Original document: {:?}", some_doc);

        // Ensure we don't accidentally "find" the document by virtue of it
        // being the first in the data file.
        for _ in 0..4 {
            docs.save(&random(), &ADocument { gubbins: random() })
                .expect("save");
        }
        docs.save(&some_id, &some_doc).expect("save");
        for _ in 0..4 {
            docs.save(&random(), &ADocument { gubbins: random() })
                .expect("save");
        }

        let loaded = docs.load(&some_id).expect("load");
        info!("Loaded document: {:?}", loaded);

        assert_eq!(Some(some_doc), loaded);
    }

    #[test]
    fn should_update_on_overwrite() {
        pretty_env_logger::try_init().unwrap_or_default();
        let pool = pool("should_update_on_overwrite");

        let some_id = random::<Id<ADocument>>();
        let some_doc = ADocument { gubbins: random() };

        let conn = pool.get().expect("temp connection");
        let docs = Documents::wrap(&*conn);

        info!("Original document: {:?}", some_doc);
        docs.save(&some_id, &some_doc).expect("save original");

        let modified_doc = ADocument { gubbins: random() };
        info!("Modified document: {:?}", modified_doc);
        docs.save(&some_id, &modified_doc).expect("save modified");

        let loaded = docs.load(&some_id).expect("load");
        info!("Loaded document: {:?}", loaded);

        assert_eq!(Some(modified_doc), loaded);
    }

    #[test]
    fn supports_transaction() {
        pretty_env_logger::try_init().unwrap_or_default();
        let pool = pool("supports_transaction");

        let some_id = random::<Id<ADocument>>();

        let conn = pool.get().expect("temp connection");
        let t = conn.transaction().expect("begin");
        let docs = Documents::wrap(&t);
        docs.save(&some_id, &ADocument { gubbins: random() })
            .expect("save");
        let _ = docs.load::<ADocument>(&some_id).expect("load");
    }

    #[test]
    fn supports_connection() {
        pretty_env_logger::try_init().unwrap_or_default();
        let pool = pool("supports_connection");

        let some_id = random::<Id<ADocument>>();

        let conn = pool.get().expect("temp connection");
        let docs = Documents::wrap(&*conn);
        docs.save(&some_id, &ADocument { gubbins: random() })
            .expect("save");
        let _ = docs.load::<ADocument>(&some_id).expect("load");
    }
}
