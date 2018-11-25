use futures::Future;
use std::time::{Duration, Instant};
use tokio::timer::Delay;
use warp;
use warp::Filter;
use {render, WithTemplate};

#[derive(Serialize, Debug, WeftTemplate)]
#[template(path = "src/menu/coffee.html")]
pub struct Coffee {
    id: u64,
    name: String,
}

pub struct Menu {}

// impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection>
impl Menu {
    pub fn handler() -> impl warp::Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> {
        warp::get2()
            .and(warp::path::end())
            .and_then(Self::index)
            .and_then(render)
    }

    fn index() -> impl Future<Item = WithTemplate<Coffee>, Error = warp::Rejection> {
        Self::index_impl().map_err(|e| warp::reject::custom(e.compat()))
    }

    fn index_impl() -> impl Future<Item = WithTemplate<Coffee>, Error = failure::Error> {
        info!("Handle index");
        let then = Instant::now() + Duration::from_millis(300);
        Delay::new(then)
            .map_err(|e| failure::Error::from(e))
            .and_then(|t| {
                info!("Timer fired: {:?}", t);

                let res = WithTemplate {
                    name: "template.html",
                    value: Coffee {
                        id: 42,
                        name: "Umbrella".into(),
                    },
                };
                futures::future::result(Ok(res))
            })
    }
}
