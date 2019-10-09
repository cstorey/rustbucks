use anyhow::{Context, Error, Result};
use log::*;

use infra::ids;
use infra::persistence::DocumentConnectionManager;

pub mod config;
pub mod menu;
pub mod orders;
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

    pub fn setup(&self) -> Result<()> {
        debug!("Init schema");
        self.db
            .get()?
            .setup()
            .with_context(|| "Setup persistence")?;
        Ok(())
    }

    pub fn menu(&self) -> Result<menu::Menu<DocumentConnectionManager>> {
        menu::Menu::new(self.db.clone())
    }
    pub fn orders(&self) -> Result<orders::Orders<DocumentConnectionManager>> {
        orders::Orders::new(self.db.clone(), self.idgen.clone())
    }
}
