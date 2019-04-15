use rand::prelude::*;
use std::sync::Arc;

use actix_web::server::{HttpHandler, HttpHandlerTask};
use actix_web::{
    App, AsyncResponder, Form, FutureResponse, HttpRequest, HttpResponse, Path, Responder, State,
};
use failure::Error;
use failure::ResultExt;
use futures::future::{lazy, poll_fn};
use futures::Future;
use r2d2::Pool;
use tokio_threadpool::{blocking, ThreadPool};

use documents::DocMeta;
use ids::Id;
use menu::Coffee;
use persistence::*;
use templates::WeftResponse;
use WithTemplate;

use super::models::Order;

const PREFIX: &'static str = "/orders";

#[derive(Debug, Clone)]
pub struct Orders {
    db: Pool<DocumentConnectionManager>,
    pool: Arc<ThreadPool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrderForm {
    coffee_id: Id<Coffee>,
}

#[derive(Debug, WeftRenderable)]
#[template(path = "src/orders/order-list.html")]
struct OrderList {}

#[derive(Debug, WeftRenderable)]
#[template(path = "src/orders/order.html")]
struct OrderWidget {
    order: Order,
}

impl Orders {
    pub fn new(db: Pool<DocumentConnectionManager>, pool: Arc<ThreadPool>) -> Result<Self, Error> {
        Ok(Orders { db, pool })
    }

    pub fn app(&self) -> Box<dyn HttpHandler<Task = Box<dyn HttpHandlerTask>>> {
        App::with_state(self.clone())
            .prefix(PREFIX)
            .resource("", |r| {
                r.name("orders");
                r.post().with(Orders::submit);
                r.get().with(Orders::list);
            })
            .resource("{id}", |r| {
                r.name("show");
                r.get().with(Orders::show)
            })
            .boxed()
    }

    fn submit((form, req): (Form<OrderForm>, HttpRequest<Self>)) -> FutureResponse<impl Responder> {
        debug!("Submit form: {:?}", form);
        req.state()
            .new_order(form.into_inner())
            .and_then(move |order_id| {
                debug!("Some order id: {}", order_id);
                let uri = req
                    .url_for("show", &[order_id.to_string()])
                    .context("url for show")?;
                Ok(HttpResponse::SeeOther()
                    .header("location", uri.to_string())
                    .finish())
            })
            .from_err()
            .responder()
    }

    fn list(_state: State<Self>) -> Result<impl Responder, Error> {
        let data = WithTemplate {
            value: OrderList {},
        };
        Ok(WeftResponse::of(data))
    }

    fn show((state, id): (State<Self>, Path<Id<Order>>)) -> FutureResponse<impl Responder> {
        let id = id.into_inner();
        state
            .load_order(id)
            .map(|orderp| {
                orderp.map(|order| {
                    let data = WithTemplate {
                        value: OrderWidget { order },
                    };
                    WeftResponse::of(data)
                })
            })
            .from_err()
            .responder()
    }

    fn new_order(&self, order: OrderForm) -> impl Future<Item = Id<Order>, Error = failure::Error> {
        self.in_pool(move |docs| {
            let id = thread_rng().gen::<Id<Order>>();
            let order = Order {
                meta: DocMeta::new_with_id(id),
                coffee_id: order.coffee_id,
            };
            docs.save(&order)?;
            debug!("Saved {:?}", order);
            Ok(id)
        })
    }

    fn load_order(
        &self,
        order_id: Id<Order>,
    ) -> impl Future<Item = Option<Order>, Error = failure::Error> {
        self.in_pool(move |docs| {
            let order = docs.load(&order_id)?;
            debug!("Load {} -> {:?}", order_id, order);
            Ok(order)
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
