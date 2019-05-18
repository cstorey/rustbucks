use std::env;

use failure::Error;
use failure::ResultExt;
use log::*;
use postgres;
use r2d2::Pool;
use r2d2_postgres::{PostgresConnectionManager, TlsMode};

use infra::persistence::{DocumentConnectionManager, Documents};

#[derive(Debug)]
struct UseTempSchema(String);

impl r2d2::CustomizeConnection<Documents, postgres::Error> for UseTempSchema {
    fn on_acquire(&self, conn: &mut Documents) -> Result<(), postgres::Error> {
        loop {
            let t = conn.get_ref().transaction()?;
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
        conn.get_ref()
            .execute(&format!("SET search_path TO \"{}\"", self.0), &[])?;
        Ok(())
    }
}

pub(crate) fn pool(schema: &str) -> Result<Pool<DocumentConnectionManager>, Error> {
    debug!("Build pool for {}", schema);
    let url = env::var("POSTGRES_URL").context("$POSTGRES_URL")?;
    debug!("Use schema name: {}", schema);
    let manager = PostgresConnectionManager::new(&*url, TlsMode::None).expect("postgres");

    let pool = r2d2::Pool::builder()
        .max_size(2)
        .connection_customizer(Box::new(UseTempSchema(schema.to_string())))
        .build(DocumentConnectionManager::new(manager))?;

    let conn = pool.get()?;
    cleanup(&conn.get_ref(), schema)?;

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
