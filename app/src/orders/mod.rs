use failure::Error;

use actix_web::server::{HttpHandler, HttpHandlerTask};
use actix_web::{App, Form, FutureResponse, Responder, State};
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
                r.post().with(Orders::handle_submit);
            })
            .boxed()
    }

    fn handle_submit(
        (form, state): (Form<OrderForm>, State<Self>),
    ) -> FutureResponse<impl Responder> {
        let fut = futures::future::ok(state.submit(form.into_inner()));
        Box::new(fut)
    }

    fn submit(&self, form: OrderForm) -> Result<String, Error> {
        debug!("Submit form: {:?}", form);

        Ok(format!("{:#?}", form))
    }
}
