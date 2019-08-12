use actix_web::web;
use failure::{Error, ResultExt};
use log::*;
use weft_derive::WeftRenderable;

pub mod config;
mod documents;
mod ids;
mod menu;
mod orders;
mod persistence;
mod templates;
mod untyped_ids;

#[derive(Debug, WeftRenderable)]
#[template(path = "src/base.html")]
pub struct WithTemplate<C> {
    value: C,
}

#[derive(Clone)]
pub struct RustBucks {
    menu: menu::Menu<persistence::DocumentConnectionManager>,
    orders: orders::Orders<persistence::DocumentConnectionManager>,
}

impl RustBucks {
    pub fn new(config: &config::Config) -> Result<Self, Error> {
        let db = config.db.build()?;

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
