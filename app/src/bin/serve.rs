extern crate log;
extern crate pretty_env_logger;
extern crate rustbucks;
extern crate warp;

fn main() {
    pretty_env_logger::init();

    println!("Hello, world!");
    let route = rustbucks::routes();

    warp::serve(route).run(([127, 0, 0, 1], 3030));
}
