use alfred::{json, Item};
use anyhow::Error;
use buildkite_workflow_lib::workflow::BuildkiteWorkflow;
use std::io::Write;
use std::process::Command;
use std::{env, io};
use structopt::StructOpt;

const SUBCOMMAND_REFRESH: &str = "refresh";
const SUBCOMMAND_OPEN: &str = "open";

#[derive(Debug, StructOpt)]
struct Opt {
    args: Vec<String>,
}

fn main() -> Result<(), Error> {
    let opt = Opt::from_args();

    let api_key = env::var("API_KEY")?;
    let database_url = env::var("DATABASE_URL")?;
    let mut wf = BuildkiteWorkflow::new(&api_key, &database_url)?;

    match opt.args.len() {
        0 => {
            let refresh = alfred::ItemBuilder::new(SUBCOMMAND_REFRESH)
                .subtitle("Refresh Cache, be patient you will be notified once complete")
                .arg(SUBCOMMAND_REFRESH)
                .into_item();
            write_items(io::stdout(), &[refresh])?
        }
        1 => {
            let arg = opt.args.get(0).unwrap();
            match arg.as_ref() {
                SUBCOMMAND_REFRESH => {
                    wf.refresh_cache()?;
                    println!("Successfully Refreshed Buildkite cache");
                }
                _ => {
                    let results = wf.query(&opt.args)?;
                    write_items(io::stdout(), &results)?
                }
            }
        }
        2 => {
            let arg = opt.args.get(0).unwrap();
            match arg.as_ref() {
                SUBCOMMAND_OPEN => {
                    Command::new("open")
                        .arg(&opt.args.get(1).unwrap())
                        .output()?;
                }
                _ => {
                    let results = wf.query(&opt.args)?;
                    write_items(io::stdout(), &results)?
                }
            }
        }
        _ => {
            let results = wf.query(&opt.args)?;
            write_items(io::stdout(), &results)?
        }
    }
    Ok(())
}

fn write_items<W>(writer: W, items: &[Item]) -> Result<(), Error>
where
    W: Write,
{
    json::write_items(writer, &items[..])?;
    Ok(())
}
