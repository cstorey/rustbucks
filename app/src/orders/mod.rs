use failure::Error;
use futures::Future;
use ids::Id;
use warp::Filter;
use {error_to_rejection, render, WithTemplate};

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

    pub fn handler(
        &self,
    ) -> impl warp::Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
        let me = self.clone();
        warp::post2()
            .and(warp::path("order"))
            .and(warp::filters::body::form::<OrderForm>())
            .and_then(move |form| error_to_rejection(me.submit(form)))
    }

    fn submit(
        &self,
        form: OrderForm,
    ) -> impl Future<Item = impl warp::Reply, Error = failure::Error> {
        futures::future::lazy(move || Ok(warp::reply::reply()))
    }
}
