#[macro_use]
extern crate log;
extern crate pretty_env_logger;
extern crate serde;
extern crate warp;
#[macro_use]
extern crate serde_derive;
extern crate failure;
#[macro_use]
extern crate weft_derive;
extern crate weft;
use warp::Filter;

#[derive(Serialize, Debug, WeftTemplate)]
#[template(path = "src/template.html")]
struct ViewData {
    id: u64,
}

#[derive(Debug, WeftTemplate)]
#[template(path = "src/base.html")]
struct WithTemplate {
    name: &'static str,
    value: ViewData,
}

fn render(template: WithTemplate) -> Result<impl warp::Reply, warp::Rejection> {
    let res = weft::render_to_string(&template);

    match res {
        Ok(s) => {
            let resp = warp::http::Response::builder()
                .header("content-type", "text/html; charset=utf8")
                .body(s);
            Ok(resp)
        }
        Err(e) => {
            error!("Could not render template {}: {}", template.name, e);
            Err(warp::reject::custom("Rendering template"))
        }
    }
}

fn handle_err(err: warp::Rejection) -> Result<impl warp::Reply, warp::Rejection> {
    error!("Handling: {:?}", err);

    Ok(warp::reply::with_status(
        "Internal Error",
        warp::http::StatusCode::INTERNAL_SERVER_ERROR,
    ))
}

pub fn routes() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> {
    let route = warp::get2()
        .and(warp::path::end())
        .map(|| {
            info!("Handle index");
            WithTemplate {
                name: "template.html",
                value: ViewData { id: 42 },
            }
        }).and_then(render)
        .recover(handle_err);

    return route;
}

#[cfg(test)]
mod test {
    struct SomethingScenario;

    struct CoffeeRequest;

    struct SomethingBarista;
    struct SomethingCashier;
    struct SomethingCustomer;

    impl SomethingScenario {
        fn new() -> Self {
            SomethingScenario
        }

        fn new_barista(&self) -> SomethingBarista {
            SomethingBarista
        }
        fn new_cashier(&self) -> SomethingCashier {
            SomethingCashier
        }
        fn new_customer(&self) -> SomethingCustomer {
            SomethingCustomer
        }
    }

    impl SomethingCustomer {
        fn requests_coffee(&self, _: &SomethingCashier) -> CoffeeRequest {
            unimplemented!("SomethingCustomer::requests_coffee")
        }
        fn pays_cashier(&self, _: &CoffeeRequest, _: &SomethingCashier) -> CoffeeRequest {
            unimplemented!("SomethingCustomer::pays_cashier")
        }
        fn cannot_pay(&self, _: &CoffeeRequest, _: &SomethingCashier) -> CoffeeRequest {
            unimplemented!("SomethingCustomer::cannot_pay")
        }
    }

    impl SomethingCashier {
        fn requests_payment_for(&self, _: &CoffeeRequest, _price: u64) {
            unimplemented!("SomethingCashier::requests_payment_for")
        }
        fn issues_refund_to(&self, _: &CoffeeRequest, _: &SomethingCustomer) {
            unimplemented!("SomethingCashier::issues_refund_to")
        }
    }

    impl SomethingBarista {
        fn prepares_coffee(&self, _: &CoffeeRequest) {
            unimplemented!("SomethingBarista::prepares_coffee")
        }

        fn delivers(&self, _: &CoffeeRequest, _: &SomethingCustomer) {
            unimplemented!("SomethingBarista::delivers")
        }

        fn disposes(&self, _: &CoffeeRequest) {
            unimplemented!("SomethingBarista::delivers")
        }

        fn has_run_out_of_milk(&self) {
            unimplemented!("SomethingBarista::has_run_out_of_milk")
        }
    }

    #[test]
    #[ignore]
    fn should_serve_coffee() {
        let scenario = SomethingScenario::new();

        let cashier = scenario.new_cashier();
        let barista = scenario.new_barista();
        let customer = scenario.new_customer();

        let req = customer.requests_coffee(&cashier);
        cashier.requests_payment_for(&req, 42);

        barista.prepares_coffee(&req);

        customer.pays_cashier(&req, &cashier);

        barista.delivers(&req, &customer);
    }

    #[test]
    #[ignore]
    fn should_abort_if_customer_cannot_pay() {
        let scenario = SomethingScenario::new();

        let cashier = scenario.new_cashier();
        let barista = scenario.new_barista();
        let customer = scenario.new_customer();

        let req = customer.requests_coffee(&cashier);
        cashier.requests_payment_for(&req, 42);

        barista.prepares_coffee(&req);

        customer.cannot_pay(&req, &cashier);

        barista.disposes(&req);
    }

    #[test]
    #[ignore]
    fn should_give_refund_if_out_of_something() {
        let scenario = SomethingScenario::new();

        let cashier = scenario.new_cashier();
        let barista = scenario.new_barista();
        let customer = scenario.new_customer();

        let req = customer.requests_coffee(&cashier);
        cashier.requests_payment_for(&req, 42);
        customer.pays_cashier(&req, &cashier);

        barista.has_run_out_of_milk();
        barista.disposes(&req);

        cashier.issues_refund_to(&req, &customer);
    }
}
