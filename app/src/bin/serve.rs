extern crate actix;
extern crate actix_web;
extern crate failure;
extern crate hyper;
#[macro_use]
extern crate log;
extern crate jemallocator;
extern crate pretty_env_logger;
extern crate rustbucks;
#[macro_use]
extern crate structopt;

use failure::ResultExt;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "serve", about = "Serve Rustbucks.")]
struct Opt {
    /// Input file
    #[structopt(parse(from_os_str))]
    config: PathBuf,
}

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

fn main() -> Result<(), failure::Error> {
    pretty_env_logger::init();
    let sys = actix::System::new("rustbucks-app");

    let opt = Opt::from_args();
    debug!("Options: {:?}", opt);

    let app = rustbucks::RustBucks::new()?;

    let srv = actix_web::server::new(move || app.app())
        .bind("0.0.0.0:3030")
        .context("bind")?;
    info!("Listening on: {:?}", srv.addrs());
    srv.start();
    let _: i32 = sys.run();
    Ok(())
}
