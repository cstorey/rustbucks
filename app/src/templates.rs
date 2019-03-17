use actix_web::{self, HttpRequest, HttpResponse, Responder};
use weft::WeftRenderable;

const TEXT_HTML: &'static str = "text/html; charset=utf-8";

pub struct WeftResponse<T>(T);

impl<T> WeftResponse<T> {
    pub fn of(val: T) -> Self {
        WeftResponse(val)
    }
}

impl<T: WeftRenderable> Responder for WeftResponse<T> {
    type Item = HttpResponse;
    type Error = actix_web::Error;

    fn respond_to<S: 'static>(self, _: &HttpRequest<S>) -> Result<Self::Item, Self::Error> {
        let WeftResponse(data) = self;
        weft::render_to_string(&data)
            .map_err(|e| actix_web::Error::from(e))
            .map(|html| HttpResponse::Ok().content_type(TEXT_HTML).body(html))
    }
}
