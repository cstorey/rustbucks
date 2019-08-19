//! Guarded with `#[cfg(test)]` from `lib.rs`

use env_logger;
use failure::Fallible;

use crate::drinker::Drinker;
use crate::ids::{Id, IdGen};
use crate::menu::Drink;
use crate::orders::Order;

mod junk_drawer;

#[test]
fn trivial_order_workflow_as_transaction_script() -> Fallible<()> {
    env_logger::try_init().unwrap_or_default();
    let pool = junk_drawer::pool("trivial_order_workflow_as_transaction_script")?;
    let conn = pool.get()?;
    let idgen = IdGen::new();
    let mut tea = Drink::new(idgen.generate(), "bubble tea");
    conn.save(&mut tea)?;

    let mut drinker = Drinker::incarnate(&idgen);
    conn.save(&mut drinker)?;

    let mut order = Order::for_drink(tea.meta.id, drinker.meta.id, &idgen);
    conn.save(&mut order)?;

    drinker.deliver_drink(order.drink_id);
    conn.save(&mut drinker)?;

    assert!(
        drinker.has_drink(tea.meta.id),
        "Drinker {:?} should have received a {:?}",
        drinker,
        tea
    );
    Ok(())
}

#[cfg(test)]
impl Drinker {
    fn has_drink(&self, drink_id: Id<Drink>) -> bool {
        self.received_drinks.contains(&drink_id)
    }

    fn deliver_drink(&mut self, drink_id: Id<Drink>) {
        self.received_drinks.insert(drink_id);
    }
}

#[test]
#[ignore]
fn trivial_order_workflow_for_two_teas() -> Fallible<()> {
    env_logger::try_init().unwrap_or_default();
    let pool = crate::persistence::test::pool("trivial_order_workflow_for_two_teas")?;
    let conn = pool.get()?;
    let idgen = IdGen::new();
    let mut tea = Drink::new(idgen.generate(), "bubble tea");
    conn.save(&mut tea)?;

    let mut drinker = Drinker::incarnate(&idgen);
    conn.save(&mut drinker)?;

    let mut order = Order::for_drink(tea.meta.id, drinker.meta.id, &idgen);
    conn.save(&mut order)?;
    drinker.deliver_drink(order.drink_id);
    conn.save(&mut drinker)?;

    drinker.deliver_drink(order.drink_id);
    conn.save(&mut drinker)?;

    assert_eq!(drinker.received_drinks.len(), 2);
    Ok(())
}

#[test]
#[ignore]
fn trivial_order_workflow_when_out_of_milk() -> Fallible<()> {
    unimplemented!("trivial_order_workflow_when_out_of_milk")
}
