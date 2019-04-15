use failure::{Error, ResultExt};
use futures::future::{lazy, poll_fn, result};
use futures::Future;
use std::sync::Arc;

use actix_web::server::{HttpHandler, HttpHandlerTask};
use actix_web::{
    http, App, AsyncResponder, FromRequest, FutureResponse, HttpRequest, HttpResponse, Path,
    Responder,
};
use r2d2::Pool;
use tokio_threadpool::{blocking, ThreadPool};

use documents::DocMeta;
use ids::Id;
use persistence::*;
use templates::WeftResponse;
use WithTemplate;

use super::models::{Coffee, CoffeeList};

const PREFIX: &'static str = "/menu";

#[derive(Debug, Clone)]
pub struct Menu {
    db: Pool<DocumentConnectionManager>,
    pool: Arc<ThreadPool>,
}

#[derive(Debug, WeftRenderable)]
#[template(path = "src/menu/menu.html")]
struct MenuWidget {
    drink: Vec<(Id<Coffee>, Coffee)>,
}
#[derive(Debug, WeftRenderable)]
#[template(path = "src/menu/drink.html")]
struct DrinkWidget {
    drink: Coffee,
}

impl Menu {
    pub fn new(db: Pool<DocumentConnectionManager>, pool: Arc<ThreadPool>) -> Result<Self, Error> {
        let conn = db.get()?;
        Self::insert(&conn, "Umbrella").context("insert umbrella")?;
        Self::insert(&conn, "Fnordy").context("insert fnordy")?;
        Ok(Menu { db, pool })
    }

    fn insert(docs: &Documents, name: &str) -> Result<(), Error> {
        let drink = {
            let id = Id::hashed(name);
            let mut drink = docs
                .load(&id)
                .context("load drink")?
                .unwrap_or_else(|| Coffee {
                    meta: DocMeta::new_with_id(id),
                    ..Default::default()
                });
            drink.name = name.into();
            docs.save(&drink).context("Save drink")?;
            drink
        };

        let list = {
            let id = CoffeeList::id();
            let mut list: CoffeeList =
                docs.load(&id)
                    .context("load list")?
                    .unwrap_or_else(|| CoffeeList {
                        meta: DocMeta::new_with_id(id),
                        ..Default::default()
                    });
            list.drinks.insert(drink.meta.id);
            docs.save(&list).context("save list")?;
            debug!("Updated list: {:?}", list);
            list
        };
        debug!("Saved drink at {:?}: {:?}", list.meta, drink);
        Ok(())
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
        result(Path::<Id<Coffee>>::extract(req))
            .and_then(move |id| {
                let id = id.into_inner();
                me.load_drink(id).from_err().map(move |drinkp| {
                    drinkp.map(|drink| {
                        WeftResponse::of(WithTemplate {
                            value: DrinkWidget { drink: drink },
                        })
                    })
                })
            })
            .responder()
    }

    fn load_menu(&self) -> impl Future<Item = Vec<(Id<Coffee>, Coffee)>, Error = failure::Error> {
        self.in_pool(move |docs| -> Result<Vec<(Id<Coffee>, Coffee)>, Error> {
            trace!("load_menu {:?}", {
                let t = ::std::thread::current();
                t.name()
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| format!("{:?}", t.id()))
            });
            let list = docs
                .load::<CoffeeList>(&CoffeeList::id())?
                .unwrap_or_default();
            let result = list
                .drinks
                .into_iter()
                .map(|id| {
                    docs.load::<Coffee>(&id)
                        .and_then(|coffeep| {
                            coffeep
                                .ok_or_else(|| failure::err_msg(format!("missing coffee? {}", &id)))
                        })
                        .map(|coffee| (id, coffee))
                })
                .collect::<Result<Vec<(Id<Coffee>, Coffee)>, Error>>()?;

            Ok(result)
        })
    }

    fn load_drink(
        &self,
        id: Id<Coffee>,
    ) -> impl Future<Item = Option<Coffee>, Error = failure::Error> {
        self.in_pool(move |docs| {
            trace!("load_drink {:?}", {
                let t = ::std::thread::current();
                t.name()
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| format!("{:?}", t.id()))
            });
            let res = docs.load(&id)?;
            debug!("Load {} -> {:?}", id, res);
            Ok(res)
        })
    }

    fn in_pool<R: Send + 'static, F: Fn(&Documents) -> Result<R, Error> + Send + 'static>(
        &self,
        f: F,
    ) -> impl Future<Item = R, Error = failure::Error> {
        let db = self.db.clone();
        let f = lazy(|| {
            poll_fn(move || {
                blocking(|| {
                    let docs = db.get()?;
                    f(&*docs)
                })
            })
            .map_err(Error::from)
        });
        self.pool.spawn_handle(f).and_then(futures::future::result)
    }
}

impl MenuWidget {
    fn drinks<'a>(&'a self) -> impl 'a + Iterator<Item = &'a (Id<Coffee>, Coffee)> {
        self.drink.iter()
    }
}
