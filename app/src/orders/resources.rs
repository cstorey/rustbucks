use actix_threadpool::BlockingError;
use actix_web::{web, HttpRequest, HttpResponse, Responder, Scope};
use failure::Error;
use futures::Future;
use r2d2::Pool;

use ids::{Id, IdGen};
use menu::Drink;
use persistence::*;
use templates::WeftResponse;
use WithTemplate;

use super::models::Order;

const PREFIX: &'static str = "/orders";

#[derive(Debug, Clone)]
pub struct Orders {
    db: Pool<DocumentConnectionManager>,
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
#[template(path = "src/orders/order.html")]
struct OrderWidget {
    order: Order,
}

impl Orders {
    pub fn new(db: Pool<DocumentConnectionManager>, idgen: IdGen) -> Result<Self, Error> {
        Ok(Orders { db, idgen })
    }

    pub fn app(&self) -> Scope {
        web::scope(PREFIX)
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
                web::get().to_async(move |id: web::Path<Id<Order>>| me.show(id.into_inner()))
            }))
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
                    .url_for("show", &[order_id.to_string()])
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
            let order = Order::for_drink(order.drink_id, &me.idgen);
            docs.save(&order)?;
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
    fn in_pool<R: Send + 'static, F: Fn(&Documents) -> Result<R, Error> + Send + 'static>(
        &self,
        f: F,
    ) -> impl Future<Item = R, Error = failure::Error> {
        let db = self.db.clone();
        web::block(move || {
            let docs = db.get()?;
            f(&*docs)
        })
        .map_err(|e| match e {
            BlockingError::Error(e) => e.into(),
            c @ BlockingError::Canceled => format_err!("{}", c),
        })
    }
}
