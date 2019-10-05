use failure::{Fallible, ResultExt};
use log::*;
use r2d2::Pool;

use infra::{documents::HasMeta, ids::Id, persistence::Storage};

mod models;
pub use models::{Drink, DrinkList};

use crate::services::{Queryable, Request};

pub struct ShowMenu;

#[derive(Debug)]
pub struct Menu<M: r2d2::ManageConnection> {
    db: Pool<M>,
}

impl<M: r2d2::ManageConnection<Connection = D>, D: Storage + Send + 'static> Menu<M> {
    pub fn new(db: Pool<M>) -> Fallible<Self> {
        Ok(Menu { db })
    }

    pub fn setup(&self) -> Fallible<()> {
        let conn = self.db.get()?;
        Self::insert(&conn, "Umbrella").context("insert umbrella")?;
        Self::insert(&conn, "Fnordy").context("insert fnordy")?;
        Ok(())
    }

    fn insert(docs: &D, name: &str) -> Fallible<()> {
        let drink = {
            let id = Id::hashed(name);
            let mut drink = docs
                .load(&id)
                .context("load drink")?
                .unwrap_or_else(|| Drink::new(id, name));
            docs.save(&mut drink).context("Save drink")?;
            drink
        };

        let list = {
            let id = DrinkList::id();
            let mut list: DrinkList = docs
                .load(&id)
                .context("load list")?
                .unwrap_or_else(|| DrinkList::new(id));
            list.drinks.insert(drink.meta().id);
            docs.save(&mut list).context("save list")?;
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
    for &Menu<M>
{
    fn query(self, _query: ShowMenu) -> Fallible<Vec<Drink>> {
        let conn = self.db.get()?;
        let list = conn
            .load(&DrinkList::id())?
            .ok_or_else(|| failure::err_msg("Missing drink list"))?;

        let mut res = Vec::new();
        for d in list.drinks.iter() {
            if let Some(drink) = conn.load(d)? {
                res.push(drink);
            }
        }

        Ok(res)
    }
}
