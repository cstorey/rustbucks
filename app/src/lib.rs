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
extern crate siphasher;
extern crate tokio_threadpool;

#[cfg(test)]
extern crate serde_json;

mod ids;
mod menu;

use actix_web::server::{HttpHandler, HttpHandlerTask};

const TEXT_HTML: &'static str = "text/html; charset=utf-8";

#[derive(Debug, WeftRenderable)]
#[template(path = "src/base.html")]
pub struct WithTemplate<C> {
    value: C,
}

// Replace with responder impl
#[cfg(never)]
fn render<C: weft::WeftRenderable + fmt::Debug>(
    template: WithTemplate<C>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let res = weft::render_to_string(&template);

    match res {
        Ok(s) => {
            let resp = warp::http::Response::builder()
                .header("content-type", "text/html; charset=utf8")
                .body(s);
            Ok(resp)
        }
        Err(e) => {
            error!("Could not render template {:?}: {}", template, e);
            Err(warp::reject::custom(e))
        }
    }
}

#[derive(Clone)]
pub struct RustBucks {
    menu: menu::Menu,
}

impl RustBucks {
    pub fn new() -> Self {
        let menu = menu::Menu::new();
        RustBucks { menu }
    }

    pub fn app(&self) -> Vec<Box<dyn HttpHandler<Task = Box<dyn HttpHandlerTask>>>> {
        info!("Booting rustbucks");
        vec![self.menu.app()]
    }
}
