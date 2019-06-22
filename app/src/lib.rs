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
extern crate actix_service;
extern crate actix_threadpool;
extern crate actix_web;
extern crate base64;
extern crate hex_slice;
extern crate postgres;
extern crate r2d2;
extern crate r2d2_postgres;
extern crate rand;
extern crate serde_json;
extern crate siphasher;
#[cfg(test)]
#[macro_use]
extern crate maplit;
extern crate env_logger;
extern crate hybrid_clocks;
#[cfg(test)]
#[macro_use]
extern crate lazy_static;
extern crate actix_files;
extern crate rustbucks_vlq as vlq;
extern crate time;

use actix_web::web;
use failure::{Error, ResultExt};

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
    menu: menu::Menu<persistence::DocumentConnectionManager>,
    orders: orders::Orders,
}

impl RustBucks {
    pub fn new(config: &config::Config) -> Result<Self, Error> {
        let db = config.postgres.build()?;

        debug!("Init schema");
        db.get()?.setup().context("Setup persistence")?;

        let idgen = ids::IdGen::new();
        let menu = menu::Menu::new(db.clone())?;
        let orders = orders::Orders::new(db.clone(), idgen)?;

        Ok(RustBucks { menu, orders })
    }

    pub fn configure(&self, cfg: &mut web::ServiceConfig) {
        let redir_root = web::resource("/").route(web::get().to_async(menu::index_redirect));
        cfg.service(redir_root);
        self.menu.configure(cfg);
        self.orders.configure(cfg);
    }
}
