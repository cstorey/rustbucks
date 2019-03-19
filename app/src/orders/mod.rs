use rand::prelude::*;

use failure::Error;

use actix_web::server::{HttpHandler, HttpHandlerTask};
use actix_web::{App, Form, FutureResponse, HttpRequest, HttpResponse, Responder, State};
use ids::Id;
use templates::WeftResponse;
use WithTemplate;

const PREFIX: &'static str = "/orders";

#[derive(Debug, Clone)]
pub struct Orders;

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct OrderForm {
    coffee_id: Id,
}

#[derive(Debug, WeftRenderable)]
#[template(path = "src/orders/order.html")]
struct OrderWidget {}

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
            .boxed()
    }

    fn submit((form, req): (Form<OrderForm>, HttpRequest<Self>)) -> Result<impl Responder, Error> {
        debug!("Submit form: {:?}", form);
        let order_id = thread_rng().gen::<Id>();
        debug!("Some order id: {}", order_id);

        Ok(HttpResponse::SeeOther()
            .header("location", req.uri().to_string())
            .finish())
    }

    fn list(_state: State<Self>) -> Result<impl Responder, Error> {
        let data = WithTemplate {
            value: OrderWidget {},
        };
        Ok(WeftResponse::of(data))
    }
}
