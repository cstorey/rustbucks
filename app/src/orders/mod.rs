use failure::Error;

use actix_web::server::{HttpHandler, HttpHandlerTask};
use actix_web::{App, Form, State};
use ids::Id;

const PREFIX: &'static str = "/orders";

#[derive(Debug, Clone)]
pub struct Orders;

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct OrderForm {
    coffee_id: Id,
}

impl Orders {
    pub fn new() -> Self {
        Orders
    }

    pub fn app(&self) -> Box<dyn HttpHandler<Task = Box<dyn HttpHandlerTask>>> {
        App::with_state(self.clone())
            .prefix(PREFIX)
            .resource("", |r| {
                r.post().with(Orders::submit);
            })
            .boxed()
    }

    fn submit((form, _): (Form<OrderForm>, State<Self>)) -> Result<String, Error> {
        debug!("Submit form: {:?}", form);

        Ok(format!("{:#?}", form))
    }
}
