//! Guarded with `#[cfg(test)]` from `lib.rs`

use env_logger;
use failure::{Error, Fallible};
use log::*;
use r2d2::Pool;
use serde::{de::DeserializeOwned, Serialize};

use crate::drinker::Drinker;
use crate::ids::{Entity, Id, IdGen};
use crate::menu::Drink;
use crate::orders::{Order, OrderDst};
use crate::persistence::{DocumentConnectionManager, Storage};
use infra::documents::HasMeta;

mod junk_drawer;

struct OrderSystem {
    pool: Pool<DocumentConnectionManager>,
}

impl OrderSystem {
    fn new(pool: Pool<DocumentConnectionManager>) -> Self {
        OrderSystem { pool }
    }

    fn store<E: Entity + Serialize + HasMeta>(&mut self, entity: &mut E) -> Fallible<()> {
        self.pool.get()?.save(entity)?;
        let meta = entity.meta();
        debug!("Stored entity: {}@{:?}", meta.id, meta.version);
        Ok(())
    }

    fn load<E: Entity + DeserializeOwned>(&mut self, id: &Id<E>) -> Fallible<Option<E>> {
        debug!("Loading entity with id: {}", id);
        Ok(self.pool.get()?.load(id)?)
    }

    fn run_until_quiescent(&self) -> Fallible<()> {
        let docs = self.pool.get()?;
        debug!("Scan for unprocessed entities");
        while let Some(mut order) = docs.load_next_unsent::<Order>()? {
            let order_id = order.meta.id;
            debug!("Found unprocessed entity: {}", order_id);
            for msg in order.mbox.drain() {
                debug!("Entity: {}; msg:{:?}", order_id, msg);
                match msg {
                    OrderDst::Barista(drinker_id, drink_id) => {
                        // This is _totally_ a massive cheat.
                        debug!("Entity: {}; do barista things", order_id);
                        let mut drinker = docs
                            .load::<Drinker>(&drinker_id)?
                            .expect("drinker not found?");
                        drinker.deliver_drink(drink_id);
                        docs.save(&mut drinker)?;
                    }
                }
            }

            docs.save(&mut order)?;
        }
        debug!("Done scan for unprocessed entities");
        Ok(())
    }
}

#[test]
fn trivial_order_workflow_as_transaction_script() -> Fallible<()> {
    env_logger::try_init().unwrap_or_default();
    let pool = junk_drawer::pool("order_workflow")?;
    let idgen = IdGen::new();
    let mut sys = OrderSystem::new(pool);

    let mut tea = Drink::new(idgen.generate(), "bubble tea");
    sys.store(&mut tea)?;

    let mut drinker = Drinker::incarnate(&idgen);
    sys.store(&mut drinker)?;

    let mut order = Order::for_drink(tea.meta.id, drinker.meta.id, &idgen);
    sys.store(&mut order)?;

    drinker.deliver_drink(order.drink_id);
    sys.store(&mut drinker)?;

    assert!(
        drinker.has_drink(tea.meta.id),
        "Drinker {:?} should have received a {:?}",
        drinker,
        tea
    );
    #[cfg(never)]
    {}
    Ok(())
}
