use failure::{Error, Fallible, ResultExt};
use log::*;

use infra::ids;
use infra::persistence::DocumentConnectionManager;

pub mod config;
pub mod menu;
mod orders;
pub mod services;

#[derive(Clone)]
pub struct RustBucks {
    db: r2d2::Pool<DocumentConnectionManager>,
    idgen: ids::IdGen,
}

impl RustBucks {
    pub fn new(config: &config::Config) -> Result<Self, Error> {
        let db = config.postgres.build()?;

        let idgen = ids::IdGen::new();

        Ok(RustBucks { db, idgen })
    }

    pub fn setup(&self) -> Fallible<()> {
        debug!("Init schema");
        self.db.get()?.setup().context("Setup persistence")?;
        Ok(())
    }

    pub fn menu(&self) -> Fallible<menu::Menu<DocumentConnectionManager>> {
        menu::Menu::new(self.db.clone())
    }
}
