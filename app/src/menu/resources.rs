use failure::{Error, ResultExt};
use futures::future::{lazy, poll_fn};
use futures::Future;
use std::sync::Arc;

use actix_web::{http, web, HttpRequest, HttpResponse, Responder, Scope};
use r2d2::Pool;
use tokio_threadpool::{blocking, ThreadPool};

use ids::Id;
use persistence::*;
use templates::WeftResponse;
use WithTemplate;

use super::models::{Drink, DrinkList};

const PREFIX: &'static str = "/menu";

#[derive(Debug, Clone)]
pub struct Menu {
    db: Pool<DocumentConnectionManager>,
    pool: Arc<ThreadPool>,
}

#[derive(Debug, WeftRenderable)]
#[template(path = "src/menu/menu.html")]
struct MenuWidget {
    drink: Vec<(Id<Drink>, Drink)>,
}
#[derive(Debug, WeftRenderable)]
#[template(path = "src/menu/drink.html")]
struct DrinkWidget {
    drink: Drink,
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
                .unwrap_or_else(|| Drink::new(id, name));
            docs.save(&drink).context("Save drink")?;
            drink
        };

        let list = {
            let id = DrinkList::id();
            let mut list: DrinkList = docs
                .load(&id)
                .context("load list")?
                .unwrap_or_else(|| DrinkList::new(id));
            list.drinks.insert(drink.meta.id);
            docs.save(&list).context("save list")?;
            debug!("Updated list: {:?}", list);
            list
        };
        debug!("Saved drink at {:?}: {:?}", list.meta, drink);
        Ok(())
    }

    pub fn app(&self) -> Scope {
        web::scope(PREFIX)
            .service({
                let me = self.clone();
                web::resource("/").route(web::get().to_async(move || me.index()))
            })
            .service({
                let me = self.clone();
                web::resource("/{id}").route(
                    web::get().to_async(move |id: web::Path<Id<Drink>>| me.detail(id.into_inner())),
                )
            })
    }

    pub fn index_redirect(req: HttpRequest) -> Result<HttpResponse, Error> {
        debug!("Redirecting from: {}", req.uri());
        let url = format!("{}/", PREFIX);
        info!("Target {} → {}", req.uri(), url);

        Ok(HttpResponse::SeeOther()
            .header(http::header::LOCATION, url)
            .finish())
    }

    fn index(&self) -> impl Future<Item = impl Responder, Error = Error> {
        info!("Handle index");
        info!("Handle from : {:?}", ::std::thread::current());
        self.load_menu().from_err().map(|menu| {
            info!("Resume from : {:?}", ::std::thread::current());
            let data = WithTemplate {
                value: MenuWidget { drink: menu },
            };
            WeftResponse::of(data)
        })
    }

    fn detail(
        &self,
        id: Id<Drink>,
    ) -> impl Future<Item = impl Responder, Error = actix_web::Error> {
        let me = self.clone();
        me.load_drink(id).from_err().map(move |drinkp| {
            drinkp.map(|drink| {
                WeftResponse::of(WithTemplate {
                    value: DrinkWidget { drink: drink },
                })
            })
        })
    }

    fn load_menu(&self) -> impl Future<Item = Vec<(Id<Drink>, Drink)>, Error = failure::Error> {
        self.in_pool(move |docs| -> Result<Vec<(Id<Drink>, Drink)>, Error> {
            trace!("load_menu {:?}", {
                let t = ::std::thread::current();
                t.name()
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| format!("{:?}", t.id()))
            });
            let list = docs
                .load::<DrinkList>(&DrinkList::id())?
                .unwrap_or_else(|| unimplemented!());
            let result = list
                .drinks
                .into_iter()
                .map(|id| {
                    docs.load::<Drink>(&id)
                        .and_then(|drinkp| {
                            drinkp
                                .ok_or_else(|| failure::err_msg(format!("missing drink? {}", &id)))
                        })
                        .map(|drink| (id, drink))
                })
                .collect::<Result<Vec<(Id<Drink>, Drink)>, Error>>()?;

            Ok(result)
        })
    }

    fn load_drink(
        &self,
        id: Id<Drink>,
    ) -> impl Future<Item = Option<Drink>, Error = failure::Error> {
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
    fn drinks<'a>(&'a self) -> impl 'a + Iterator<Item = &'a (Id<Drink>, Drink)> {
        self.drink.iter()
    }
}
