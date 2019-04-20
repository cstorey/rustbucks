use std::time::Duration;

use failure::{Error, ResultExt};
use r2d2::Pool;
use r2d2_postgres::{PostgresConnectionManager, TlsMode};

use persistence;

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct Config {
    pub postgres: PgConfig,
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct PgConfig {
    pub url: String,
    max_size: Option<u32>,
    min_idle: Option<u32>,
    max_lifetime: Option<Duration>,
    idle_timeout: Option<Duration>,
    connection_timeout: Option<Duration>,
}
impl PgConfig {
    pub(crate) fn build(&self) -> Result<Pool<persistence::DocumentConnectionManager>, Error> {
        debug!("Build pool from {:?}", self);

        let manager = persistence::DocumentConnectionManager::new(
            PostgresConnectionManager::new(&*self.url, TlsMode::None)
                .context("connection manager")?,
        );

        let mut builder = r2d2::Pool::builder();

        if let Some(max_size) = self.max_size {
            builder = builder.max_size(max_size);
        }
        if let Some(min_idle) = self.min_idle {
            builder = builder.min_idle(Some(min_idle));
        }
        if let Some(max_lifetime) = self.max_lifetime {
            builder = builder.max_lifetime(Some(max_lifetime));
        }
        if let Some(idle_timeout) = self.idle_timeout {
            builder = builder.idle_timeout(Some(idle_timeout));
        }
        if let Some(connection_timeout) = self.connection_timeout {
            builder = builder.connection_timeout(connection_timeout);
        }

        debug!("Pool builder: {:?}", builder);
        let pool = builder.build(manager).context("build pool")?;

        Ok(pool)
    }
}
