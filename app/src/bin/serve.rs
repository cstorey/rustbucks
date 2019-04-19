extern crate actix;
extern crate actix_web;
extern crate failure;
extern crate hyper;
#[macro_use]
extern crate log;
extern crate jemallocator;
extern crate pretty_env_logger;
extern crate rustbucks;
extern crate structopt;
extern crate toml;
#[macro_use]
extern crate serde_derive;

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use failure::ResultExt;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "serve", about = "Serve Rustbucks.")]
struct Opt {
    /// Input file
    #[structopt(parse(from_os_str))]
    config: PathBuf,
}

#[derive(Deserialize, Debug)]
struct Config {
    #[serde(flatten)]
    rustbucks: rustbucks::Config,
    listener: Listener,
}

#[derive(Deserialize, Debug)]
struct Listener {
    addr: std::net::SocketAddr,
}

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

fn main() -> Result<(), failure::Error> {
    pretty_env_logger::init();
    let sys = actix::System::new("rustbucks-app");

    let opt = Opt::from_args();
    debug!("Options: {:?}", opt);

    let mut config_buf = String::new();
    File::open(&opt.config)?.read_to_string(&mut config_buf)?;
    let config: Config = toml::from_str(&config_buf)?;

    let app = rustbucks::RustBucks::new(&config.rustbucks)?;

    let srv = actix_web::server::new(move || app.app())
        .bind(&config.listener.addr)
        .context("bind")?;
    info!("Listening on: {:?}", srv.addrs());
    srv.start();
    let _: i32 = sys.run();
    Ok(())
}
