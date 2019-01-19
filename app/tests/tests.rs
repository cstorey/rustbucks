extern crate base64;
extern crate byteorder;
extern crate failure;
extern crate futures;
extern crate log;
extern crate pretty_env_logger;
extern crate rustbucks;
extern crate serde;
extern crate siphasher;
extern crate sulfur;
extern crate tokio;
extern crate tokio_threadpool;
extern crate warp;
#[macro_use]
extern crate lazy_static;

use failure::Error;
use std::net::SocketAddr;
use std::sync::Mutex;
use sulfur::{chrome, By};
use tokio::runtime;

lazy_static! {
    static ref RT: Mutex<runtime::Runtime> =
        Mutex::new(runtime::Runtime::new().expect("tokio runtime"));
}
struct SomethingScenario {
    shutdown: Option<futures::sync::oneshot::Sender<()>>,
    addr: SocketAddr,
}

struct CoffeeRequest;

struct SomethingBarista;
struct SomethingCashier {}
struct SomethingCustomer {
    browser: sulfur::DriverHolder,
    url: String,
}

impl SomethingScenario {
    fn new() -> Result<Self, Error> {
        let (shutdown, trigger) = futures::sync::oneshot::channel::<()>();
        let (addr, server) = warp::serve(rustbucks::routes())
            .bind_with_graceful_shutdown(([127, 0, 0, 1], 0), trigger);
        println!("Listening on: {}", addr);
        RT.lock().expect("lock runtime").spawn(server);
        Ok(SomethingScenario {
            shutdown: Some(shutdown),
            addr: addr,
        })
    }

    fn new_barista(&self) -> Result<SomethingBarista, Error> {
        SomethingBarista::new()
    }
    fn new_cashier(&self) -> Result<SomethingCashier, Error> {
        SomethingCashier::new()
    }
    fn new_customer(&self) -> Result<SomethingCustomer, Error> {
        SomethingCustomer::new(&self.url())
    }
    fn url(&self) -> String {
        format!("http://{}/", self.addr)
    }
}

impl SomethingCustomer {
    fn new(url: &str) -> Result<SomethingCustomer, Error> {
        let browser = chrome::start(chrome::Config::default().headless(true))?;
        let url = url.to_string();
        Ok(SomethingCustomer { browser, url })
    }

    fn requests_coffee(&self, _: &SomethingCashier) -> Result<CoffeeRequest, Error> {
        self.browser.visit(&self.url)?;

        let a_coffee_elt = self.browser.find_element(&By::css(".a-coffee"))?;
        self.browser.click(&a_coffee_elt)?;

        let order_button = self.browser.find_element(&By::css("button.order"))?;
        self.browser.click(&order_button)?;
        // TODO: Actually extract _some_ kind of reference?
        Ok(CoffeeRequest)
    }

    fn pays_cashier(&self, _: &CoffeeRequest, _: &SomethingCashier) -> CoffeeRequest {
        unimplemented!("SomethingCustomer::pays_cashier")
    }
    fn cannot_pay(&self, _: &CoffeeRequest, _: &SomethingCashier) -> CoffeeRequest {
        unimplemented!("SomethingCustomer::cannot_pay")
    }
}

impl SomethingCashier {
    fn new() -> Result<Self, Error> {
        Ok(SomethingCashier {})
    }

    fn requests_payment_for(&self, _: &CoffeeRequest, _price: u64) -> Result<(), Error> {
        // TODO
        Ok(())
    }
    fn issues_refund_to(&self, _: &CoffeeRequest, _: &SomethingCustomer) {
        unimplemented!("SomethingCashier::issues_refund_to")
    }
}

impl Drop for SomethingScenario {
    fn drop(&mut self) {
        self.shutdown
            .take()
            .expect("shutdown trigger")
            .send(())
            .expect("send cashier shutdown")
    }
}

impl SomethingBarista {
    fn new() -> Result<Self, Error> {
        Ok(SomethingBarista {})
    }

    fn prepares_coffee(&self, _: &CoffeeRequest) {
        // Visits the barista UI
        // Finds the named request
        // Presses buttons to do things
        // Confirms coffee made
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
fn should_serve_coffee_partial() {
    pretty_env_logger::try_init().unwrap_or(());

    let scenario = SomethingScenario::new().expect("new scenario");

    let cashier = scenario.new_cashier().expect("new cashier");
    let _barista = scenario.new_barista().expect("new barista");
    let customer = scenario.new_customer().expect("new customer");

    let req = customer.requests_coffee(&cashier).expect("requests coffee");
    cashier
        .requests_payment_for(&req, 42)
        .expect("requested payment");
}

#[test]
#[ignore]
fn should_serve_coffee() {
    pretty_env_logger::try_init().unwrap_or(());

    let scenario = SomethingScenario::new().expect("new scenario");

    let cashier = scenario.new_cashier().expect("new cashier");
    let barista = scenario.new_barista().expect("new barista");;
    let customer = scenario.new_customer().expect("new customer");

    let req = customer.requests_coffee(&cashier).expect("requests coffee");

    cashier
        .requests_payment_for(&req, 42)
        .expect("requested payment");
    barista.prepares_coffee(&req);
    customer.pays_cashier(&req, &cashier);
    barista.delivers(&req, &customer);
}

#[test]
#[ignore]
fn should_abort_if_customer_cannot_pay() {
    let scenario = SomethingScenario::new().expect("new scenario");

    let cashier = scenario.new_cashier().expect("new cashier");
    let barista = scenario.new_barista().expect("new barista");;
    let customer = scenario.new_customer().expect("new customer");

    let req = customer.requests_coffee(&cashier).expect("requests coffee");
    cashier
        .requests_payment_for(&req, 42)
        .expect("requested payment");

    barista.prepares_coffee(&req);

    customer.cannot_pay(&req, &cashier);

    barista.disposes(&req);
}

#[test]
#[ignore]
fn should_give_refund_if_out_of_something() {
    let scenario = SomethingScenario::new().expect("new scenario");

    let cashier = scenario.new_cashier().expect("new cashier");
    let barista = scenario.new_barista().expect("new barista");;
    let customer = scenario.new_customer().expect("new customer");

    let req = customer.requests_coffee(&cashier).expect("requests coffee");
    cashier
        .requests_payment_for(&req, 42)
        .expect("requested payment");
    customer.pays_cashier(&req, &cashier);

    barista.has_run_out_of_milk();
    barista.disposes(&req);

    cashier.issues_refund_to(&req, &customer);
}