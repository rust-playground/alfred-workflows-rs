use alfred::{json, Item};
use anyhow::Error;
use buildkite_workflow_lib::workflow::Workflow;
use clap::{Parser, Subcommand};
use std::io::Write;
use std::process::Command;
use std::{env, io};

const SUBCOMMAND_REFRESH: &str = "refresh";

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[arg()]
    name: Option<Vec<String>>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Refresh,
    Open { url: String },
}

fn main() -> Result<(), Error> {
    let opts = Cli::parse();

    let api_key = env::var("API_KEY")?;
    let database_url = env::var("DATABASE_URL")?;
    let mut wf = Workflow::new(&api_key, &database_url)?;

    match opts.command {
        Some(Commands::Refresh) => {
            wf.refresh_cache()?;
            println!("Successfully Refreshed Buildkite cache");
        }
        Some(Commands::Open { url }) => {
            Command::new("open").arg(url).output()?;
        }
        _ => {
            if let Some(name_parts) = opts.name {
                let results = wf.query(&name_parts)?;
                write_items(io::stdout(), &results)?;
            } else {
                let refresh = alfred::ItemBuilder::new(SUBCOMMAND_REFRESH)
                    .subtitle("Refresh Cache, be patient you will be notified once complete")
                    .arg(SUBCOMMAND_REFRESH)
                    .into_item();
                write_items(io::stdout(), &[refresh])?;
            }
        }
    }
    Ok(())
}

fn write_items<W>(writer: W, items: &[Item]) -> Result<(), Error>
where
    W: Write,
{
    json::write_items(writer, items)?;
    Ok(())
}
