use futures::future::{lazy, poll_fn};
use futures::Future;
use tokio_threadpool::blocking;

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
        info!("Handle from : {:?}", ::std::thread::current());
        let f = lazy(|| {
            poll_fn(move || {
                blocking(|| {
                    use std::thread;
                    info!("Hello from : {:?}", thread::current());
                    ()
                })
            })
        });
        let _: &Future<Item = (), Error = tokio_threadpool::BlockingError> = &f;
        let f = f.map_err(|e| failure::Error::from(e));
        let f = f.and_then(|()| {
            info!("Resume from : {:?}", ::std::thread::current());
            let res = WithTemplate {
                name: "template.html",
                value: Coffee {
                    id: 42,
                    name: "Umbrella".into(),
                },
            };
            futures::future::result(Ok(res))
        });
        f
    }
}
