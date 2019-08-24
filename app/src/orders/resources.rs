use actix_threadpool::BlockingError;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use failure::Error;
use futures::Future;
use log::*;
use r2d2::Pool;
use serde::Deserialize;
use weft_derive::WeftRenderable;

use crate::menu::Drink;
use crate::templates::WeftResponse;
use crate::WithTemplate;
use infra::ids::{Id, IdGen};
use infra::persistence::*;
use infra::untyped_ids::UntypedId;

use super::models::Order;

const PREFIX: &str = "/orders";

#[derive(Debug)]
pub struct Orders<M: r2d2::ManageConnection> {
    db: Pool<M>,
    idgen: IdGen,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrderForm {
    drink_id: Id<Drink>,
}

#[derive(Debug, WeftRenderable)]
#[template(path = "src/orders/order-list.html")]
struct OrderList {}

#[derive(Debug, WeftRenderable)]
#[template(path = "src/orders/order.html", selector = "#content")]
struct OrderWidget {
    order: Order,
}

impl<M: r2d2::ManageConnection<Connection = D>, D: Storage + Send + 'static> Orders<M> {
    pub fn new(db: Pool<M>, idgen: IdGen) -> Result<Self, Error> {
        Ok(Orders { db, idgen })
    }

    pub fn configure(&self, cfg: &mut web::ServiceConfig) {
        let scope = web::scope(PREFIX)
            .service(
                web::resource("")
                    .name("orders")
                    .route({
                        let me = self.clone();
                        web::get().to_async(move || me.list())
                    })
                    .route({
                        let me = self.clone();
                        web::post().to_async(
                            move |(form, req): (web::Form<OrderForm>, HttpRequest)| {
                                me.submit(form.into_inner(), req)
                            },
                        )
                    }),
            )
            .service(web::resource("{id}").name("show").route({
                let me = self.clone();
                web::get()
                    .to_async(move |id: web::Path<UntypedId>| me.show(id.into_inner().typed()))
            }));
        cfg.service(scope);
    }

    fn submit(
        &self,
        form: OrderForm,
        req: HttpRequest,
    ) -> impl Future<Item = impl Responder, Error = failure::Error> {
        debug!("Submit form: {:?}", form);
        self.new_order(form)
            .and_then(move |order_id| {
                debug!("Some order id: {}", order_id);
                let uri = req
                    .url_for("show", &[order_id.untyped().to_string()])
                    .map_err(|e| failure::err_msg(e.to_string()))?;
                Ok(HttpResponse::SeeOther()
                    .header("location", uri.to_string())
                    .finish())
            })
            .from_err()
    }

    fn list(&self) -> Result<impl Responder, Error> {
        let data = WithTemplate {
            value: OrderList {},
        };
        Ok(WeftResponse::of(data))
    }

    fn show(&self, id: Id<Order>) -> impl Future<Item = impl Responder, Error = Error> {
        self.load_order(id)
            .map(|orderp| {
                orderp.map(|order| {
                    let data = WithTemplate {
                        value: OrderWidget { order },
                    };
                    WeftResponse::of(data)
                })
            })
            .from_err()
    }

    fn new_order(&self, order: OrderForm) -> impl Future<Item = Id<Order>, Error = failure::Error> {
        let me = self.clone();
        self.in_pool(move |docs| {
            let mut order = Order::for_drink(order.drink_id, &me.idgen);
            docs.save(&mut order)?;
            debug!("Saved {:?}", order);
            Ok(order.meta.id)
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
    fn in_pool<R: Send + 'static, F: Fn(&D) -> Result<R, Error> + Send + 'static>(
        &self,
        f: F,
    ) -> impl Future<Item = R, Error = failure::Error> {
        let db = self.db.clone();
        web::block(move || {
            let docs = db.get()?;
            f(&*docs)
        })
        .map_err(|e| match e {
            BlockingError::Error(e) => e,
            c @ BlockingError::Canceled => failure::format_err!("{}", c),
        })
    }
}

impl<M: r2d2::ManageConnection> Clone for Orders<M> {
    fn clone(&self) -> Self {
        let db = self.db.clone();
        let idgen = self.idgen.clone();
        Orders { db, idgen }
    }
}
