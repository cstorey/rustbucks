use rand::prelude::*;

use actix_web::server::{HttpHandler, HttpHandlerTask};
use actix_web::{
    App, AsyncResponder, Form, FutureResponse, HttpRequest, HttpResponse, Path, Responder, State,
};
use failure::Error;
use failure::ResultExt;
use futures::Future;
use ids::{Entity, Id};
use menu::Coffee;
use templates::WeftResponse;
use WithTemplate;

const PREFIX: &'static str = "/orders";

#[derive(Debug, Clone)]
pub struct Orders;

#[derive(Debug, Clone, Deserialize)]
pub struct OrderForm {
    coffee_id: Id<Coffee>,
}

struct Order;

impl Entity for Order {
    const PREFIX: &'static str = "order";
}

#[derive(Debug, WeftRenderable)]
#[template(path = "src/orders/order-list.html")]
struct OrderList {}

#[derive(Debug, WeftRenderable)]
#[template(path = "src/orders/order.html")]
struct OrderWidget {
    id: Id<Order>,
}

impl Orders {
    pub fn new() -> Self {
        Orders
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

    fn new_order(&self, order: OrderForm) -> impl Future<Item = Id<Order>, Error = failure::Error> {
        let order_id = thread_rng().gen::<Id<Order>>();
        futures::future::ok(order_id)
    }

    fn list(_state: State<Self>) -> Result<impl Responder, Error> {
        let data = WithTemplate {
            value: OrderList {},
        };
        Ok(WeftResponse::of(data))
    }

    fn show((_state, id): (State<Self>, Path<Id<Order>>)) -> Result<impl Responder, Error> {
        let id = id.into_inner();
        let data = WithTemplate {
            value: OrderWidget { id: id },
        };
        Ok(WeftResponse::of(data))
    }
}
