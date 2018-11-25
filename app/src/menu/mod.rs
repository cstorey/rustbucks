use failure::Error;
use futures::future::{lazy, poll_fn};
use futures::Future;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio_threadpool::blocking;

use warp;
use warp::Filter;
use {render, WithTemplate};

#[derive(Serialize, Debug, Clone, Hash)]
pub struct Coffee {
    name: String,
}

#[derive(Debug, Clone)]
pub struct Menu {
    drinks: Arc<HashMap<u64, Coffee>>,
}

#[derive(Serialize, Debug, WeftRenderable)]
#[template(path = "src/menu/menu.html")]
struct MenuWidget {
    drink: Vec<(u64, Coffee)>,
}

// impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection>
impl Menu {
    pub fn new() -> Self {
        let mut map = HashMap::new();
        Self::insert(
            &mut map,
            Coffee {
                name: "Umbrella".into(),
            },
        );
        Self::insert(
            &mut map,
            Coffee {
                name: "Fnordy".into(),
            },
        );
        Menu {
            drinks: Arc::new(map),
        }
    }

    fn insert(map: &mut HashMap<u64, Coffee>, drink: Coffee) {
        let id = {
            let mut h = siphasher::sip::SipHasher24::new();
            drink.hash(&mut h);
            h.finish()
        };

        let prev_size = map.len();
        map.insert(id, drink);
        assert!(map.len() > prev_size);
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
        let f = self.load_menu();
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

    fn load_menu(&self) -> impl Future<Item = Vec<(u64, Coffee)>, Error = failure::Error> {
        let me = self.clone();
        lazy(|| {
            poll_fn(move || {
                blocking(|| {
                    use std::thread;
                    info!("Hello from : {:?}", thread::current());
                    me.drinks
                        .iter()
                        .map(|(id, d)| (id.clone(), d.clone()))
                        .collect::<Vec<(u64, Coffee)>>()
                })
            }).map_err(Error::from)
        })
    }
}

impl MenuWidget {
    fn drinks<'a>(&'a self) -> impl 'a + Iterator<Item = &'a (u64, Coffee)> {
        self.drink.iter()
    }
}
