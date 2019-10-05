use failure::{Error, ResultExt};
use log::*;

use infra::ids;

pub mod config;
mod menu;
mod orders;

#[derive(Clone)]
pub struct RustBucks {
}

impl RustBucks {
    pub fn new(config: &config::Config) -> Result<Self, Error> {
        let db = config.postgres.build()?;

        debug!("Init schema");
        db.get()?.setup().context("Setup persistence")?;

        let idgen = ids::IdGen::new();

        Ok(RustBucks { })
    }
}
