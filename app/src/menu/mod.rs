use failure::Error;
use futures::future::{lazy, poll_fn};
use futures::Future;
use tokio_threadpool::blocking;

use warp;
use warp::Filter;
use {render, WithTemplate};

#[derive(Serialize, Debug)]
pub struct Coffee {
    id: u64,
    name: String,
}

#[derive(Debug, Clone)]
pub struct Menu {}

#[derive(Serialize, Debug, WeftRenderable)]
#[template(path = "src/menu/menu.html")]
struct MenuWidget {
    drink: Coffee,
}

// impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection>
impl Menu {
    pub fn new() -> Self {
        Menu {}
    }
    pub fn handler(
        &self,
    ) -> impl warp::Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> {
        let me = self.clone();
        warp::get2()
            .and(warp::path::end())
            .and_then(move || me.index())
            .and_then(render)
    }

    fn index(&self) -> impl Future<Item = WithTemplate<MenuWidget>, Error = warp::Rejection> {
        self.index_impl()
            .map_err(|e| warp::reject::custom(e.compat()))
    }

    fn index_impl(&self) -> impl Future<Item = WithTemplate<MenuWidget>, Error = failure::Error> {
        info!("Handle index");
        info!("Handle from : {:?}", ::std::thread::current());
        let f = Self::load_menu();
        let f = f.map_err(|e| failure::Error::from(e));
        let f = f.and_then(|menu| {
            info!("Resume from : {:?}", ::std::thread::current());
            let res = WithTemplate {
                name: "template.html",
                value: MenuWidget { drink: menu },
            };
            futures::future::result(Ok(res))
        });
        f
    }

    fn load_menu() -> impl Future<Item = Coffee, Error = failure::Error> {
        lazy(|| {
            poll_fn(move || {
                blocking(|| {
                    use std::thread;
                    info!("Hello from : {:?}", thread::current());
                    Coffee {
                        id: 42,
                        name: "Umbrella".into(),
                    }
                })
            }).map_err(Error::from)
        })
    }
}

impl MenuWidget {
    fn drinks<'a>(&'a self) -> impl 'a + Iterator<Item = &'a Coffee> {
        vec![&self.drink].into_iter()
    }
}
