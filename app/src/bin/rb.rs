use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use failure::Fallible;
use serde::Deserialize;
use structopt::StructOpt;

use infra::documents::HasMeta;
use rustbucks::{menu::ShowMenu, services::Queryable};

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
}

#[derive(Deserialize, Debug)]
struct Config {
    #[serde(flatten)]
    rustbucks: rustbucks::config::Config,
    env_logger: rustbucks::config::EnvLogger,
}

fn main() -> Fallible<()> {
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
    }

    Ok(())
}
