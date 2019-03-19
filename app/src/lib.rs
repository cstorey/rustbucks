#[macro_use]
extern crate log;
extern crate futures;
extern crate pretty_env_logger;
extern crate serde;
extern crate tokio;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate failure;
extern crate weft;
#[macro_use]
extern crate weft_derive;
extern crate actix_web;
extern crate base64;
extern crate byteorder;
extern crate rand;
extern crate siphasher;
extern crate tokio_threadpool;

#[cfg(test)]
extern crate serde_json;

mod ids;
mod menu;
mod orders;
mod templates;

use actix_web::server::{HttpHandler, HttpHandlerTask};
use actix_web::App;

#[derive(Debug, WeftRenderable)]
#[template(path = "src/base.html")]
pub struct WithTemplate<C> {
    value: C,
}

#[derive(Clone)]
pub struct RustBucks {
    menu: menu::Menu,
    orders: orders::Orders,
}

impl RustBucks {
    pub fn new() -> Self {
        let menu = menu::Menu::new();
        let orders = orders::Orders::new();
        RustBucks { menu, orders }
    }

    pub fn app(&self) -> Vec<Box<dyn HttpHandler<Task = Box<dyn HttpHandlerTask>>>> {
        info!("Booting rustbucks");

        let redir_root = App::new().resource("/", |r| r.get().f(menu::Menu::index_redirect));
        vec![self.menu.app(), self.orders.app(), redir_root.boxed()]
    }
}
