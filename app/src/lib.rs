#[macro_use]
extern crate log;
extern crate futures;
extern crate pretty_env_logger;
extern crate serde;
extern crate tokio;
extern crate warp;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate failure;
extern crate weft;
#[macro_use]
extern crate weft_derive;
extern crate base64;
extern crate byteorder;
extern crate siphasher;
extern crate tokio_threadpool;

use std::fmt;

use warp::Filter;

mod ids;
mod menu;

#[derive(Debug, WeftRenderable)]
#[template(path = "src/base.html")]
pub struct WithTemplate<C> {
    value: C,
}

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

fn log_err(err: warp::Rejection) -> Result<&'static str, warp::Rejection> {
    error!("Saw error: {:?}", err);
    Err(err)
}

pub fn routes() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> {
    let menu = menu::Menu::new();
    menu.handler().recover(log_err)
}

pub(crate) fn error_to_rejection<T>(
    f: impl futures::Future<Item = T, Error = failure::Error>,
) -> impl futures::Future<Item = T, Error = warp::Rejection> {
    f.map_err(|e| warp::reject::custom(e.compat()))
}
