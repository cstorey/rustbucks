use failure::{Error, ResultExt};
use futures::future::{lazy, poll_fn, result};
use futures::Future;
use std::sync::Arc;
use std::collections::BTreeSet;

use tokio_threadpool::{blocking, ThreadPool};
use actix_web::server::{HttpHandler, HttpHandlerTask};
use actix_web::{
    http, App, AsyncResponder, FromRequest, FutureResponse, HttpRequest, HttpResponse, Path,
    Responder,
};
use postgres;
use r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager;

use ids::Id;
use persistence::Documents;
use templates::WeftResponse;
use WithTemplate;

const PREFIX: &'static str = "/menu";

#[derive(Deserialize, Serialize, Debug, Clone, Hash)]
pub struct Coffee {
    name: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct CoffeeList {
    drinks: BTreeSet<Id>,
}

#[derive(Debug, Clone)]
pub struct Menu {
    db: Pool<PostgresConnectionManager>,
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
    pub fn new(pool: Pool<PostgresConnectionManager>) -> Result<Self, Error> {
        let conn = pool.get()?;
        Self::insert(
            &conn,
            Coffee {
                name: "Umbrella".into(),
            },
        ).context("insert umbrella")?;
        Self::insert(
            &conn,
            Coffee {
                name: "Fnordy".into(),
            },
        ).context("insert fnordy")?;
        Ok(Menu {
            db: pool,
            pool: Arc::new(ThreadPool::new()),
        })
    }

    fn insert(conn: &postgres::Connection, drink: Coffee) -> Result<(), Error> {
        let id = Id::hashed(&drink);
        let t = conn.transaction()?;
        {
            let docs = Documents::wrap(&t);
            docs.save(&id, &drink).context("Save drink")?;
            let mut list: CoffeeList = docs
                .load(&CoffeeList::id())
                .context("load list")?
                .unwrap_or_default();
            list.drinks.insert(id);
            docs.save(&CoffeeList::id(), &list).context("save list")?;
            debug!("Updated list: {:?}", list);
        }
        t.commit().context("commit")?;

        debug!("Saved drink at {}: {:?}", id, drink);
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
        self.in_pool(move || -> Result<Vec<(Id, Coffee)>, Error> {
            trace!("load_menu {:?}", {
                let t = ::std::thread::current();
                t.name()
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| format!("{:?}", t.id()))
            });
            let conn = me.db.get()?;
            let t = conn.transaction()?;
            let result = {
            let docs = Documents::wrap(&t);
            let list = docs
                .load::<CoffeeList>(&CoffeeList::id())?
                .unwrap_or_default();
            list.drinks
                .into_iter()
                .map(|id| {
                    docs.load::<Coffee>(&id)
                        .and_then(|coffeep| {
                            coffeep
                                .ok_or_else(|| failure::err_msg(format!("missing coffee? {}", &id)))
                        })
                        .map(|coffee| (id, coffee))
                })
                .collect::<Result<Vec<(Id, Coffee)>, Error>>()?
            };
            t.commit()?;
            Ok(result)
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
            let conn = me.db.get()?;
            let res = Documents::wrap(&*conn).load(&id)?;
            debug!("Load {} -> {:?}", id, res);
            Ok(res)
        })
    }

    fn in_pool<R: Send + 'static, F: Fn() -> Result<R, Error> + Send + 'static>(
        &self,
        f: F,
    ) -> impl Future<Item = R, Error = failure::Error> {
        let f = lazy(|| poll_fn(move || blocking(&f)).map_err(Error::from));
        self.pool.spawn_handle(f).and_then(futures::future::result)
    }
}

impl CoffeeList {
    fn id() -> Id {
        Id::hashed(&"CoffeeList")
    }
}

impl MenuWidget {
    fn drinks<'a>(&'a self) -> impl 'a + Iterator<Item = &'a (Id, Coffee)> {
        self.drink.iter()
    }
}
