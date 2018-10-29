extern crate hyper;
extern crate log;
extern crate pretty_env_logger;
extern crate rustbucks;
extern crate warp;

fn main() {
    pretty_env_logger::init();

    let route = rustbucks::routes();

    let srv = warp::serve(route);
    let (addr, fut) = srv.bind_ephemeral(([127, 0, 0, 1], 3030));
    println!("Listening on: {}", addr);
    hyper::rt::run(fut);
}
