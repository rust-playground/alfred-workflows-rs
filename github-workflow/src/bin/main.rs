use alfred::{json, Item};
use anyhow::{anyhow, Error};
use clap::{Parser, Subcommand};
use github_workflow_lib::workflow::Workflow;
use std::io::Write;
use std::{env, io, process::Command};

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
            println!("Successfully Refreshed GitHub cache");
        }
        Some(Commands::Open { url }) => {
            Command::new("open")
                .arg(url)
                .output()
                .map_err(|e| anyhow!("failed to execute process: {}", e))?;
        }
        _ => {
            if let Some(mut name_parts) = opts.name {
                let search_str = if name_parts.len() == 1 {
                    name_parts.swap_remove(0)
                } else {
                    name_parts.join(" ")
                };
                let search_str = search_str.trim();

                let items = wf.query(search_str)?;
                write_items(io::stdout(), &items)?;
            } else {
                let refresh = alfred::ItemBuilder::new(SUBCOMMAND_REFRESH)
                    .subtitle("Refresh Cache, be patient you will be notified once complete")
                    .arg(SUBCOMMAND_REFRESH)
                    .into_item();
                write_items(io::stdout(), &[refresh])?;
            }
        }
    };
    Ok(())
}

fn write_items<W>(writer: W, items: &[Item]) -> Result<(), Error>
where
    W: Write,
{
    json::write_items(writer, items)
        .map_err(|e| anyhow!("failed to write alfred items->json: {}", e))
}
