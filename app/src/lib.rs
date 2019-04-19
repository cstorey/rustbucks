#[macro_use]
extern crate log;
extern crate futures;
extern crate pretty_env_logger;
extern crate serde;
extern crate tokio;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate failure;
extern crate weft;
#[macro_use]
extern crate weft_derive;
extern crate actix_web;
extern crate base64;
extern crate hex_slice;
extern crate postgres;
extern crate r2d2;
extern crate r2d2_postgres;
extern crate rand;
extern crate serde_json;
extern crate siphasher;
extern crate tokio_threadpool;
#[cfg(test)]
#[macro_use]
extern crate maplit;

use std::sync::Arc;
use std::time::Duration;

use actix_web::server::{HttpHandler, HttpHandlerTask};
use actix_web::App;
use failure::{Error, ResultExt};
use r2d2::Pool;
use r2d2_postgres::{PostgresConnectionManager, TlsMode};
use tokio_threadpool::ThreadPool;

mod documents;
mod ids;
mod menu;
mod orders;
mod persistence;
mod templates;

#[derive(Debug, WeftRenderable)]
#[template(path = "src/base.html")]
pub struct WithTemplate<C> {
    value: C,
}

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

#[derive(Clone)]
pub struct RustBucks {
    menu: menu::Menu,
    orders: orders::Orders,
}

impl RustBucks {
    pub fn new(config: &Config) -> Result<Self, Error> {
        let db = config.postgres.build()?;

        debug!("Init schema");
        db.get()?.setup().context("Setup persistence")?;

        let threads = Arc::new(ThreadPool::new());
        let menu = menu::Menu::new(db.clone(), threads.clone())?;
        let orders = orders::Orders::new(db.clone(), threads.clone())?;

        Ok(RustBucks { menu, orders })
    }

    pub fn app(&self) -> Vec<Box<dyn HttpHandler<Task = Box<dyn HttpHandlerTask>>>> {
        info!("Booting rustbucks");

        let redir_root = App::new().resource("/", |r| r.get().f(menu::Menu::index_redirect));
        vec![self.menu.app(), self.orders.app(), redir_root.boxed()]
    }
}

impl PgConfig {
    fn build(&self) -> Result<Pool<persistence::DocumentConnectionManager>, Error> {
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
