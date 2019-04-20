extern crate actix;
extern crate actix_web;
extern crate base64;
extern crate failure;
extern crate futures;
extern crate log;
extern crate pretty_env_logger;
extern crate rustbucks;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate siphasher;
extern crate sulfur;
extern crate tokio;
extern crate tokio_threadpool;
#[macro_use]
extern crate lazy_static;
extern crate envy;

use std::env;
use std::net::SocketAddr;

use actix_web::test;
use failure::Error;
use failure::ResultExt;
use sulfur::{chrome, By};

#[derive(Deserialize, Debug)]
struct TestConfig {
    headless: Option<bool>,
}

lazy_static! {
    static ref TEST_CONFIG: TestConfig = envy::prefixed("TESTS_")
        .from_env()
        .expect("Load test environment");
    static ref CHROME_CONFIG: sulfur::chrome::Config = {
        chrome::Config::default()
            .headless(TEST_CONFIG.headless.unwrap_or(true))
            .clone()
    };
}

struct SomethingScenario {
    _srv: test::TestServer,
    addr: SocketAddr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DrinkRequest(String);

struct SomethingBarista {
    browser: sulfur::DriverHolder,
    url: String,
}
struct SomethingCashier {}
struct SomethingCustomer {
    browser: sulfur::DriverHolder,
    url: String,
}

impl SomethingScenario {
    fn new() -> Result<Self, Error> {
        let mut config = rustbucks::config::Config::default();
        config.postgres.url = env::var("POSTGRES_URL").context("$POSTGRES_URL")?;
        let app = rustbucks::RustBucks::new(&config).expect("new rustbucks");

        let _srv = test::TestServer::with_factory(move || app.app());

        let addr = _srv.addr();
        println!("Listening on: {:?}", addr);

        Ok(SomethingScenario { _srv, addr })
    }

    fn new_barista(&self) -> Result<SomethingBarista, Error> {
        SomethingBarista::new(&self.url())
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
        let browser = chrome::start(&CHROME_CONFIG)?;
        let url = url.to_string();
        Ok(SomethingCustomer { browser, url })
    }

    fn requests_drink(&self, _: &SomethingCashier) -> Result<DrinkRequest, Error> {
        self.browser.visit(&self.url)?;

        let a_drink_elt = self.browser.find_element(&By::css(".a-drink"))?;
        self.browser.click(&a_drink_elt)?;

        let order_button = self.browser.find_element(&By::css("button.order"))?;
        self.browser.click(&order_button)?;

        let elt = self
            .browser
            .find_element(&By::css("*[data-order-id]"))
            .expect("find request id");
        let id = self
            .browser
            .attribute(&elt, "data-order-id")
            .expect("find data-order-id")
            .expect("some data-order-id");
        Ok(DrinkRequest(id))
    }

    fn pays_cashier(&self, _: &DrinkRequest, _: &SomethingCashier) -> DrinkRequest {
        unimplemented!("SomethingCustomer::pays_cashier")
    }
    fn cannot_pay(&self, _: &DrinkRequest, _: &SomethingCashier) -> DrinkRequest {
        unimplemented!("SomethingCustomer::cannot_pay")
    }
}

impl SomethingCashier {
    fn new() -> Result<Self, Error> {
        Ok(SomethingCashier {})
    }

    fn requests_payment_for(&self, _: &DrinkRequest, _price: u64) -> Result<(), Error> {
        // TODO
        Ok(())
    }
    fn issues_refund_to(&self, _: &DrinkRequest, _: &SomethingCustomer) {
        unimplemented!("SomethingCashier::issues_refund_to")
    }
}

impl SomethingBarista {
    fn new(url: &str) -> Result<Self, Error> {
        let browser = chrome::start(&CHROME_CONFIG)?;
        let url = url.to_string();
        Ok(SomethingBarista { browser, url })
    }
    fn prepares_drink(&self, _: &DrinkRequest) -> Result<(), Error> {
        // Visits the barista UI
        self.browser.visit(&self.url)?;
        // Finds the named request
        // Presses buttons to do things
        // Confirms drink made
        unimplemented!("SomethingBarista::prepares_drink")
    }

    fn delivers(&self, _: &DrinkRequest, _: &SomethingCustomer) {
        unimplemented!("SomethingBarista::delivers")
    }

    fn disposes(&self, _: &DrinkRequest) {
        unimplemented!("SomethingBarista::delivers")
    }

    fn has_run_out_of_milk(&self) {
        unimplemented!("SomethingBarista::has_run_out_of_milk")
    }
}

#[test]
fn should_serve_drink_partial() {
    pretty_env_logger::try_init().unwrap_or(());

    let scenario = SomethingScenario::new().expect("new scenario");

    let cashier = scenario.new_cashier().expect("new cashier");
    let _barista = scenario.new_barista().expect("new barista");;
    let customer = scenario.new_customer().expect("new customer");

    let _req = customer.requests_drink(&cashier).expect("requests drink");
}

#[test]
#[ignore]
fn should_serve_drink() {
    pretty_env_logger::try_init().unwrap_or(());

    let scenario = SomethingScenario::new().expect("new scenario");

    let cashier = scenario.new_cashier().expect("new cashier");
    let barista = scenario.new_barista().expect("new barista");;
    let customer = scenario.new_customer().expect("new customer");

    let req = customer.requests_drink(&cashier).expect("requests drink");

    cashier
        .requests_payment_for(&req, 42)
        .expect("requested payment");
    barista.prepares_drink(&req).expect("prepares_drink");
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

    let req = customer.requests_drink(&cashier).expect("requests drink");
    cashier
        .requests_payment_for(&req, 42)
        .expect("requested payment");

    barista.prepares_drink(&req).expect("prepares_drink");

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

    let req = customer.requests_drink(&cashier).expect("requests drink");
    cashier
        .requests_payment_for(&req, 42)
        .expect("requested payment");
    customer.pays_cashier(&req, &cashier);

    barista.has_run_out_of_milk();
    barista.disposes(&req);

    cashier.issues_refund_to(&req, &customer);
}
