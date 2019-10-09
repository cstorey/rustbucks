use anyhow::Result;
use chrono::{DateTime, SecondsFormat, Utc};
use infra::ids::IdGen;
use infra::untyped_ids::UntypedId;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "idgen", about = "Generate Identifiers")]
enum Commands {
    #[structopt(name = "gen", about = "Generate Identifiers")]
    Generate(Generate),
    #[structopt(name = "hash", about = "Generate via Hashing")]
    Hash(Hash),
    #[structopt(name = "decompose", about = "Decompose Identifiers")]
    Decompose(Decompose),
}

#[derive(Debug, StructOpt)]
struct Generate {
    #[structopt(short = "n", long = "count", default_value = "1")]
    count: usize,
}

#[derive(Debug, StructOpt)]
struct Hash {
    inputs: Vec<String>,
}

#[derive(Debug, StructOpt)]
struct Decompose {
    ids: Vec<UntypedId>,
}

fn main() -> Result<()> {
    let cmd = Commands::from_args();

    match cmd {
        Commands::Generate(opt) => {
            let idgen = IdGen::new();
            for _ in 0..opt.count {
                println!("{}", idgen.untyped());
            }
        }
        Commands::Hash(opt) => {
            for inp in opt.inputs.iter() {
                let id = UntypedId::hashed(inp.as_bytes());
                println!("{}", id);
            }
        }

        Commands::Decompose(opt) => {
            for id in opt.ids {
                let stamp: DateTime<Utc> = id.timestamp().into();
                let random = id.random();
                println!(
                    "t:{}; r:0x{:0>16x}",
                    stamp.to_rfc3339_opts(SecondsFormat::Nanos, true),
                    random
                );
            }
        }
    }

    Ok(())
}
