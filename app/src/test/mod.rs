//! Guarded with `#[cfg(test)]` from `lib.rs`

use failure::Fallible;
use r2d2::Pool;
use serde::Serialize;

use crate::drinker::Drinker;
use crate::ids::{Entity, IdGen};
use crate::menu;
use crate::orders;
use crate::persistence::DocumentConnectionManager;
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
        let docs = self.pool.get()?;
        docs.save(entity)?;
        Ok(())
    }
}

#[test]
fn order_workflow() -> Fallible<()> {
    let pool = junk_drawer::pool("order_workflow")?;
    let idgen = IdGen::new();
    let mut sys = OrderSystem::new(pool);

    let mut drinker = Drinker::incarnate(&idgen);
    sys.store(&mut drinker)?;

    let tea = menu::Drink::new(idgen.generate(), "bubble tea");

    let mut order = orders::Order::for_drink(tea.meta.id, drinker.meta.id, &idgen);
    sys.store(&mut order)?;

    #[cfg(never)]
    {
        assert!(
            drinker.has_drink(&item),
            "Drinker {:?} should have received a {:?}",
            drinker,
            item
        );
    }
    Ok(())
}

#[cfg(never)]
mod scratch {
    impl UnitOfWork {
        fn with_entity<E: Entity, R, F: FnMut(&mut E) -> Result<R, Error>>(&self, id: Id<E>, f: F) {
            Self::retry_on_concurrency_error(|| {
                let mut ent = self.docs.load(id)?;
                let res = f(&mut ent)?;
                self.docs.save(&mut ent)?;
                res
            })
        }

        fn retry_on_concurrency_error<R, F: Fn() -> Result<R, Error>>(f: F) {
            loop {
                match f().rescue::<ConcurrencyError>() {
                    Ok(_vers) => return res,
                    Err(e) => {
                        if let Some(_) = e.root_cause::<ConcurrencyError>() {
                        } else {
                            return Err(e);
                        }
                    }
                }
            }
        }
    }
}
