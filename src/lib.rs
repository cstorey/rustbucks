#[cfg(test)]
#[cfg(never)]
mod test {
    #[test]
    fn should_serve_coffee() {
        let scenario = SomethingScenario::new();

        let cashier = scenario.new_cashier();
        let barista = scenario.new_barista();
        let customer = scenario.new_customer();

        let req = customer.requests_coffee();
        cashier.requsts_payment_for(req, 42);

        barista.prepares_coffee(req);

        customer.pays_cashier(req, cashier);

        barista.delivers(req, customer);
    }

    #[test]
    fn should_abort_if_customer_cannot_pay() {
        let scenario = SomethingScenario::new();

        let cashier = scenario.new_cashier();
        let barista = scenario.new_barista();
        let customer = scenario.new_customer();

        let req = customer.requests_coffee();
        cashier.requsts_payment_for(req, 42);

        barista.prepares_coffee(req);

        customer.cannot_pay(req, cashier);

        barista.disposes(req);
    }

    #[test]
    fn should_give_refund_if_out_of_something() {
        let scenario = SomethingScenario::new();

        let cashier = scenario.new_cashier();
        let barista = scenario.new_barista();
        let customer = scenario.new_customer();

        let req = customer.requests_coffee();
        cashier.requsts_payment_for(req, 42);
        customer.pays_cashier(req, cashier);

        barista.barista_runs_out_of_milk(req);
        barista.disposes(req);

        cashier.issues_refund_to(req, customer);
    }
}
