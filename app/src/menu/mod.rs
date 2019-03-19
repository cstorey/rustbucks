use failure::Error;
use futures::future::{lazy, poll_fn, result};
use futures::Future;
use std::collections::HashMap;
use std::sync::Arc;
use tokio_threadpool::{blocking, ThreadPool};

use actix_web::server::{HttpHandler, HttpHandlerTask};
use actix_web::{
    http, App, AsyncResponder, FromRequest, FutureResponse, HttpRequest, HttpResponse, Path,
    Responder,
};

use ids::Id;
use templates::WeftResponse;
use WithTemplate;

const PREFIX: &'static str = "/menu";

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
        let id = Id::hashed(&drink);
        let prev_size = map.len();
        map.insert(id, drink);
        assert!(map.len() > prev_size);
    }

    pub fn app(&self) -> Box<dyn HttpHandler<Task = Box<dyn HttpHandlerTask>>> {
        App::with_state(self.clone())
            .prefix(PREFIX)
            .resource("/", |r| {
                r.get().f(move |req| req.state().index(req));
            })
            .resource("/{id}", |r| {
                r.get()
                    .f(move |req: &HttpRequest<Self>| req.state().detail(req));
            })
            .boxed()
    }

    pub fn index_redirect(req: &HttpRequest) -> Result<HttpResponse, Error> {
        debug!("Redirecting from: {}", req.uri());
        let url = format!("{}/", PREFIX);
        info!("Target {} → {}", req.uri(), url);

        Ok(HttpResponse::SeeOther()
            .header(http::header::LOCATION, url)
            .finish())
    }

    fn index(&self, _: &HttpRequest<Self>) -> FutureResponse<impl Responder> {
        info!("Handle index");
        info!("Handle from : {:?}", ::std::thread::current());
        self.load_menu()
            .from_err()
            .map(|menu| {
                info!("Resume from : {:?}", ::std::thread::current());
                let data = WithTemplate {
                    value: MenuWidget { drink: menu },
                };
                WeftResponse::of(data)
            })
            .responder()
    }

    fn detail(&self, req: &HttpRequest<Self>) -> FutureResponse<impl Responder> {
        let me = self.clone();
        result(Path::<Id>::extract(req))
            .and_then(move |id| {
                let id = id.into_inner();
                me.load_drink(id).from_err().map(move |drinkp| {
                    drinkp.map(|drink| {
                        WeftResponse::of(WithTemplate {
                            value: DrinkWidget {
                                id: id,
                                drink: drink,
                            },
                        })
                    })
                })
            })
            .responder()
    }

    fn load_menu(&self) -> impl Future<Item = Vec<(Id, Coffee)>, Error = failure::Error> {
        let me = self.clone();
        self.in_pool(move || {
            trace!("load_menu {:?}", {
                let t = ::std::thread::current();
                t.name()
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| format!("{:?}", t.id()))
            });
            me.drinks
                .iter()
                .map(|(id, d)| (id.clone(), d.clone()))
                .collect::<Vec<(Id, Coffee)>>()
        })
    }

    fn load_drink(&self, id: Id) -> impl Future<Item = Option<Coffee>, Error = failure::Error> {
        let me = self.clone();
        self.in_pool(move || {
            trace!("load_drink {:?}", {
                let t = ::std::thread::current();
                t.name()
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| format!("{:?}", t.id()))
            });
            let res = me.drinks.get(&id).map(|d| d.clone());
            debug!("Load {} -> {:?}", id, res);
            res
        })
    }

    fn in_pool<R: Send + 'static, F: Fn() -> R + Send + 'static>(
        &self,
        f: F,
    ) -> impl Future<Item = R, Error = failure::Error> {
        let f = lazy(|| poll_fn(move || blocking(&f)).map_err(Error::from));
        self.pool.spawn_handle(f)
    }
}

impl MenuWidget {
    fn drinks<'a>(&'a self) -> impl 'a + Iterator<Item = &'a (Id, Coffee)> {
        self.drink.iter()
    }
}
