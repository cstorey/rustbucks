use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use anyhow::Result;
use serde::Deserialize;
use structopt::StructOpt;

use infra::{documents::HasMeta, ids::Id};
use rustbucks::{
    menu::{Drink, ShowMenu},
    orders::{Order, PlaceOrder, QueryOrder},
    services::{Commandable, Queryable},
};

#[derive(Debug, StructOpt)]
#[structopt(name = "serve", about = "Serve Rustbucks.")]
struct Opt {
    /// Input file
    #[structopt(parse(from_os_str))]
    config: PathBuf,
    #[structopt(subcommand)]
    command: Commands,
}

#[derive(Debug, StructOpt)]
#[structopt(name = "rb", about = "Rustbucks CLI")]
enum Commands {
    #[structopt(name = "setup", about = "Initialize")]
    Setup,
    #[structopt(name = "show-menu", about = "Show menu")]
    ShowMenu,
    #[structopt(name = "order", about = "Place order")]
    Order(PlaceOrderCmd),
    OrderStatus(OrderStatus),

    #[structopt(
        name = "process-order",
        about = "Process a single outstanding order action"
    )]
    ActionOrder,
    #[structopt(
        name = "process-barista",
        about = "Process a single outstanding barista action"
    )]
    ActionBarista,
}

#[derive(Debug, StructOpt)]
struct PlaceOrderCmd {
    drink_id: Id<Drink>,
}

#[derive(Debug, StructOpt)]
struct OrderStatus {
    order_id: Id<Order>,
}

#[derive(Deserialize, Debug)]
struct Config {
    #[serde(flatten)]
    rustbucks: rustbucks::config::Config,
    env_logger: rustbucks::config::EnvLogger,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();

    let mut config_buf = String::new();
    File::open(&opt.config)?.read_to_string(&mut config_buf)?;
    let config: Config = toml::from_str(&config_buf)?;

    config.env_logger.builder().init();

    let rb = rustbucks::RustBucks::new(&config.rustbucks)?;

    match opt.command {
        Commands::Setup => {
            rb.setup()?;
            rb.menu()?.setup()?;
        }
        Commands::ShowMenu => {
            let list = rb.menu()?.query(ShowMenu)?;
            for drink in list {
                println!("{}: {}", drink.meta().id, drink.name);
            }
        }
        Commands::Order(PlaceOrderCmd { drink_id }) => {
            let order_id = rb.orders()?.execute(PlaceOrder { drink_id })?;
            println!("{}", order_id);
        }
        Commands::OrderStatus(OrderStatus { order_id }) => {
            let status = rb.orders()?.query(QueryOrder { order_id })?;
            println!(
                "Order status: id:{}; made:{:?}",
                status.order_id, status.is_made
            );
        }
        Commands::ActionOrder => {
            rb.order_worker()?.process_action()?;
        }
        Commands::ActionBarista => {
            rb.barista_worker()?.process_action()?;
        }
    }

    Ok(())
}
