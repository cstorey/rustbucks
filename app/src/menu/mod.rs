use anyhow::{Context, Result};
use log::*;
use r2d2::Pool;

use infra::{documents::HasMeta, ids::Id, persistence::Storage};

mod models;
pub use models::{Drink, DrinkList};

use crate::services::{Queryable, Request};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ShowMenu;

#[derive(Debug)]
pub struct Menu<M: r2d2::ManageConnection> {
    db: Pool<M>,
}

impl<M: r2d2::ManageConnection<Connection = D>, D: Storage + Send + 'static> Menu<M> {
    pub fn new(db: Pool<M>) -> Result<Self> {
        Ok(Menu { db })
    }

    pub fn setup(&self) -> Result<()> {
        let conn = self.db.get()?;
        Self::insert(&conn, "Umbrella").with_context(|| "insert umbrella")?;
        Self::insert(&conn, "Fnordy").with_context(|| "insert fnordy")?;
        Ok(())
    }

    fn insert(docs: &D, name: &str) -> Result<()> {
        let drink = {
            let id = Id::hashed(name);
            let mut drink = docs
                .load(&id)
                .with_context(|| "load drink")?
                .unwrap_or_else(|| Drink::new(id, name));
            docs.save(&mut drink).with_context(|| "Save drink")?;
            drink
        };

        let list = {
            let id = DrinkList::id();
            let mut list: DrinkList = docs
                .load(&id)
                .with_context(|| "load list")?
                .unwrap_or_else(|| DrinkList::new(id));
            list.drinks.insert(drink.meta().id);
            docs.save(&mut list).with_context(|| "save list")?;
            debug!("Updated list: {:?}", list);
            list
        };
        debug!("Saved drink at {:?}: {:?}", list.meta, drink);
        Ok(())
    }
}

impl Request for ShowMenu {
    type Resp = Vec<Drink>;
}

impl<M: r2d2::ManageConnection<Connection = D>, D: Storage + Send + 'static> Queryable<ShowMenu>
    for Menu<M>
{
    fn query(&self, _query: ShowMenu) -> Result<Vec<Drink>> {
        let conn = self.db.get()?;
        let list = conn.load(&DrinkList::id())?.expect("Missing drink list");

        let mut res = Vec::new();
        for d in list.drinks.iter() {
            if let Some(drink) = conn.load(d)? {
                res.push(drink);
            }
        }

        Ok(res)
    }
}
