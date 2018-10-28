#[macro_use]
extern crate log;
extern crate pretty_env_logger;
extern crate tera;
#[macro_use]
extern crate lazy_static;
extern crate serde;
extern crate warp;
#[macro_use]
extern crate serde_derive;
extern crate failure;
use std::fmt;
use tera::Tera;
use warp::Filter;

macro_rules! static_template {
    ($tera: expr, $fname: expr) => {
        $tera.add_raw_template($fname, include_str!($fname))
    };
}

lazy_static! {
    pub static ref TERA: Tera = {
        let mut tera = Tera::default();
        // tera.add_raw_template("template", include_str!("template.html")) .expect("add template");
        static_template!(tera, "../base.html").expect("base.html");
        static_template!(tera, "../template.html").expect("template.html");
        tera
    };
}

#[derive(Serialize, Debug)]
struct ViewData {
    id: u64,
}

#[derive(Debug)]
struct WithTemplate<T> {
    name: &'static str,
    value: T,
}

fn render<T: serde::Serialize + fmt::Debug>(
    template: WithTemplate<T>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let res = TERA.render(template.name, &template.value);

    match res {
        Ok(s) => {
            let resp = warp::http::Response::builder()
                .header("content-type", "text/html; charset=utf8")
                .body(s);
            Ok(resp)
        }
        Err(e) => {
            error!("Could not render template {}: {}", template.name, e);
            Err(warp::reject::custom("Rendering template"))
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

fn routes() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> {
    let route = warp::get2()
        .and(warp::path::end())
        .map(|| {
            info!("Handle index");
            WithTemplate {
                name: "template.html",
                value: ViewData { id: 42 },
            }
        }).and_then(render)
        .recover(handle_err);

    return route;
}

fn main() {
    pretty_env_logger::init();

    println!("Hello, world!");
    let route = routes();

    warp::serve(route).run(([127, 0, 0, 1], 3030));
}
