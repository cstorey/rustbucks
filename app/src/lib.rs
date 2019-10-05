use failure::{Error, Fallible, ResultExt};
use log::*;

use infra::ids;

pub mod config;
mod menu;
mod orders;

#[derive(Clone)]
pub struct RustBucks {
    db: r2d2::Pool<infra::persistence::DocumentConnectionManager>,
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
}
