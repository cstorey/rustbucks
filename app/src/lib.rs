#[macro_use]
extern crate log;
extern crate futures;
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
extern crate env_logger;

use std::sync::Arc;

use actix_web::server::{HttpHandler, HttpHandlerTask};
use actix_web::App;
use failure::{Error, ResultExt};
use tokio_threadpool::ThreadPool;

pub mod config;
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

#[derive(Clone)]
pub struct RustBucks {
    menu: menu::Menu,
    orders: orders::Orders,
}

impl RustBucks {
    pub fn new(config: &config::Config) -> Result<Self, Error> {
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
