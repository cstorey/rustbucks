#[macro_use]
extern crate log;
extern crate futures;
extern crate pretty_env_logger;
extern crate serde;
extern crate tokio;
extern crate warp;
#[macro_use]
extern crate serde_derive;
extern crate failure;
#[macro_use]
extern crate weft_derive;
extern crate weft;
use warp::Filter;

mod menu;

#[cfg(test)]
mod tests;

#[derive(Debug, WeftTemplate)]
#[template(path = "src/base.html")]
pub struct WithTemplate<C> {
    name: &'static str,
    value: C,
}

fn render<C: weft::Renderable>(
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
            error!("Could not render template {}: {}", template.name, e);
            Err(warp::reject::custom(e))
        }
    }
}

fn handle_err(err: warp::Rejection) -> Result<impl warp::Reply, warp::Rejection> {
    error!("Handling: {:?}", err);

    Ok(warp::reply::with_status(
        "Internal Error",
        warp::http::StatusCode::INTERNAL_SERVER_ERROR,
    ))
}

pub fn routes() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> {
    menu::Menu::handler().recover(handle_err)
}
