//! Guarded with `#[cfg(test)]` from `lib.rs`

use env_logger;
use failure::Fallible;

use crate::drinker::Drinker;
use crate::ids::{Id, IdGen};
use crate::menu::Drink;
use crate::orders::Order;
use crate::product::Product;
use infra::documents::{DocMeta, HasMeta};

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

    let made_drink = order.make(&idgen);
    assert_eq!(made_drink.recipe, tea.meta.id);
    drinker.deliver_drink(&made_drink);
    conn.save(&mut drinker)?;

    assert!(
        drinker.received_drinks.contains(&made_drink.meta().id),
        "Drinker {:?} should have received a {:?}",
        drinker,
        tea
    );
    Ok(())
}

#[cfg(never)]
#[test]
fn behavioral_sketch() -> Fallible<()> {
    // this, this is kinda what I started with in `tests/`.
    // Think about how we can use something like
    // https://docs.rs/tower-service/0.2.0/tower_service/trait.Service.html
    env_logger::try_init().unwrap_or_default();
    let pool = crate::persistence::test::pool("behavioral_sketch")?;
    let idgen = IdGen::new();

    let menus = MenuService::new(pool, idgen)?;
    let orders = OrderService::new(pool, idgen)?;

    let tea_recipe = menus
        .list_recipes()?
        .into_iter()
        .find(|d| drink.name == "Bubble Tea")
        .ok_or_else(|| failure::err_msg("No bubble tea?"))?;

    let order = orders.place_order(tea_recipe.meta().id)?;

    orders.pay_for_order(order)?;
    while !orders.is_ready(order)? {
        pause();
    }
    let drink = orders.take(order)?;
}

#[cfg(test)]
impl Drinker {
    fn has_drink(&self, drink_id: Id<Product>) -> bool {
        self.received_drinks.contains(&drink_id)
    }

    fn deliver_drink(&mut self, drink: &Product) {
        self.received_drinks.insert(drink.meta().id);
    }
}

impl Order {
    fn make(&self, idgen: &IdGen) -> Product {
        let id = idgen.generate();
        let meta = DocMeta::new_with_id(id);
        let recipe = self.drink_id;
        Product { meta, recipe }
    }
}

#[test]
#[ignore]
fn trivial_order_workflow_for_two_teas() -> Fallible<()> {
    env_logger::try_init().unwrap_or_default();
    let pool = junk_drawer::pool("trivial_order_workflow_for_two_teas")?;
    let conn = pool.get()?;
    let idgen = IdGen::new();

    let mut tea = Drink::new(idgen.generate(), "bubble tea");
    conn.save(&mut tea)?;

    let mut drinker = Drinker::incarnate(&idgen);
    conn.save(&mut drinker)?;

    let mut order = Order::for_drink(tea.meta.id, drinker.meta.id, &idgen);
    conn.save(&mut order)?;
    drinker.deliver_drink(&order.make(&idgen));
    conn.save(&mut drinker)?;

    drinker.deliver_drink(&order.make(&idgen));
    conn.save(&mut drinker)?;

    assert_eq!(drinker.received_drinks.len(), 2);
    Ok(())
}

#[test]
#[ignore]
fn trivial_order_workflow_when_out_of_milk() -> Fallible<()> {
    unimplemented!("trivial_order_workflow_when_out_of_milk")
}
