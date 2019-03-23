extern crate actix;
extern crate actix_web;
extern crate failure;
extern crate hyper;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
extern crate rustbucks;

use failure::ResultExt;

fn main() -> Result<(), failure::Error> {
    pretty_env_logger::init();
    let sys = actix::System::new("rustbucks-app");

    let app = rustbucks::RustBucks::new()?;

    let srv = actix_web::server::new(move || app.app())
        .bind("0.0.0.0:3030")
        .context("bind")?;
    info!("Listening on: {:?}", srv.addrs());
    srv.start();
    let _: i32 = sys.run();
    Ok(())
}
