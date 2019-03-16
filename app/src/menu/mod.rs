use failure::Error;
use futures::future::{lazy, poll_fn, result};
use futures::Future;
use std::collections::HashMap;
use std::sync::Arc;
use tokio_threadpool::{blocking, ThreadPool};

use actix_web::server::{HttpHandler, HttpHandlerTask};
use actix_web::{
    App, AsyncResponder, FromRequest, FutureResponse, HttpRequest, HttpResponse, Path,
};

use ids::Id;
use {WithTemplate, TEXT_HTML};

#[derive(Serialize, Debug, Clone, Hash)]
pub struct Coffee {
    name: String,
}

#[derive(Debug, Clone)]
pub struct Menu {
    drinks: Arc<HashMap<Id, Coffee>>,
    pool: Arc<ThreadPool>,
}

#[derive(Debug, WeftRenderable)]
#[template(path = "src/menu/menu.html")]
struct MenuWidget {
    drink: Vec<(Id, Coffee)>,
}
#[derive(Debug, WeftRenderable)]
#[template(path = "src/menu/drink.html")]
struct DrinkWidget {
    id: Id,
    drink: Coffee,
}

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
            pool: Arc::new(ThreadPool::new()),
        }
    }

    fn insert(map: &mut HashMap<Id, Coffee>, drink: Coffee) {
        let id = Id::of(&drink);
        let prev_size = map.len();
        map.insert(id, drink);
        assert!(map.len() > prev_size);
    }

    pub fn app(&self) -> Box<dyn HttpHandler<Task = Box<dyn HttpHandlerTask>>> {
        App::with_state(self.clone())
            .prefix("/menu")
            .resource("/", |r| r.get().f(move |req| req.state().index(req)))
            .resource("/{id}", |r| {
                r.get()
                    .f(move |req: &HttpRequest<Self>| req.state().detail(req))
            })
            .boxed()
    }

    fn index(&self, _: &HttpRequest<Self>) -> FutureResponse<HttpResponse> {
        info!("Handle index");
        info!("Handle from : {:?}", ::std::thread::current());
        self.load_menu()
            .from_err()
            .and_then(|menu| {
                info!("Resume from : {:?}", ::std::thread::current());
                let data = WithTemplate {
                    value: MenuWidget { drink: menu },
                };
                let html = weft::render_to_string(&data)?;
                Ok(HttpResponse::Ok().content_type(TEXT_HTML).body(html))
            })
            .responder()
    }

    fn detail(&self, req: &HttpRequest<Self>) -> FutureResponse<HttpResponse> {
        let me = self.clone();
        result(Path::<Id>::extract(req))
            .and_then(move |id| {
                let id = id.into_inner();
                me.load_drink(id)
                    .from_err()
                    .and_then(move |drinkp| {
                        drinkp.ok_or_else(|| actix_web::error::ErrorNotFound(id))
                    })
                    .and_then(move |drink| {
                        let html = weft::render_to_string(&WithTemplate {
                            value: DrinkWidget {
                                id: id,
                                drink: drink,
                            },
                        })?;
                        Ok(HttpResponse::Ok().content_type(TEXT_HTML).body(html))
                    })
            })
            .responder()
    }

    // I can either start a tokio thread pool, or I can use actix's SyncArbiter.
    // ... Okay.
    fn load_menu(&self) -> impl Future<Item = Vec<(Id, Coffee)>, Error = failure::Error> {
        let me = self.clone();
        let f = lazy(|| {
            poll_fn(move || {
                blocking(|| {
                    me.drinks
                        .iter()
                        .map(|(id, d)| (id.clone(), d.clone()))
                        .collect::<Vec<(Id, Coffee)>>()
                })
            })
            .map_err(Error::from)
        });
        self.pool.spawn_handle(f)
    }

    fn load_drink(&self, id: Id) -> impl Future<Item = Option<Coffee>, Error = failure::Error> {
        let me = self.clone();
        let f = lazy(move || {
            poll_fn(move || {
                blocking(|| {
                    let res = me.drinks.get(&id).map(|d| d.clone());
                    debug!("Load {} -> {:?}", id, res);
                    res
                })
            })
            .map_err(Error::from)
        });
        self.pool.spawn_handle(f)
    }
}

impl MenuWidget {
    fn drinks<'a>(&'a self) -> impl 'a + Iterator<Item = &'a (Id, Coffee)> {
        self.drink.iter()
    }
}
