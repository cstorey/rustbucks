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
