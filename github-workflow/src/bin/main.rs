use alfred::{json, Item};
use anyhow::{anyhow, Error};
use clap::{command, crate_authors, crate_description, crate_name, crate_version, Arg, SubCommand};
use github_workflow_lib::workflow::Workflow;
use std::borrow::Cow;
use std::io::Write;
use std::{env, io, process::Command};

const SUBCOMMAND_SETTINGS: &str = ">settings";
const SUBCOMMAND_REFRESH: &str = "refresh";
const SUBCOMMAND_OPEN: &str = "open";
const ARG_INPUT: &str = "input";

fn main() -> Result<(), Error> {
    let matches = command!("\n")
        .author(crate_authors!("\n"))
        .about(crate_description!())
        .version(crate_version!())
        .name(crate_name!())
        .allow_external_subcommands(true)
        .subcommand(
            SubCommand::with_name(SUBCOMMAND_OPEN)
                .about("opens the provided argument (https://)")
                .arg(
                    Arg::with_name(ARG_INPUT)
                        // .long(ARG_INPUT)
                        .help("the input value to open")
                        .index(1),
                ),
        )
        .subcommand(
            SubCommand::with_name(SUBCOMMAND_SETTINGS)
                .about("settings to control")
                .subcommand(
                    SubCommand::with_name(SUBCOMMAND_REFRESH).about("refreshes the cached data"),
                ),
        )
        .get_matches();

    let api_key = env::var("API_KEY")?;
    let database_url = env::var("DATABASE_URL")?;
    let mut wf = Workflow::new(&api_key, &database_url)?;

    match matches.subcommand() {
        Some((SUBCOMMAND_SETTINGS, m)) => match m.subcommand() {
            Some((SUBCOMMAND_REFRESH, _)) => {
                wf.refresh_cache()?;
                println!("Successfully Refreshed GitHub cache");
                Ok(())
            }
            _ => Err(anyhow!("No suitable SubCommand found")),
        },
        Some((SUBCOMMAND_OPEN, m)) => {
            let input = m.value_of(ARG_INPUT).unwrap_or_default();
            if input.starts_with("https://") {
                Command::new("open")
                    .arg(input)
                    .output()
                    .map_err(|e| anyhow!("failed to execute process: {}", e))?;
            }
            Ok(())
        }
        Some((external, m)) => {
            let query = match m.get_many("") {
                Some(args) => args
                    .into_iter()
                    .cloned()
                    .collect::<Vec<String>>()
                    .join(" ")
                    .into(),
                None => Cow::Borrowed(external),
            };
            // let query: Cow<'a, str> = m
            //     .get_many::<String>("")
            //     .map(|vals| vals.collect::<Vec<_>>())
            //     .unwrap_or_else(|| Cow::Borrowed(external))
            //     .join(" ");
            // let query: Vec<&str> = match m.get_many("") {
            //     Some(args) => Cow::Owned(
            //         args.vals
            //             .iter()
            //             .map(|s| s.to_string_lossy().into_owned())
            //             .collect::<Vec<String>>()
            //             .join(" "),
            //     ),
            //     None => Cow::Borrowed(external),
            // };
            let items = wf.query(&query)?;
            write_items(io::stdout(), &items)
        }
        _ => {
            let refresh = alfred::ItemBuilder::new(SUBCOMMAND_REFRESH)
                .subtitle("Refresh Cache, be patient you will be notified once complete")
                .arg(format!("{} {}", SUBCOMMAND_SETTINGS, SUBCOMMAND_REFRESH))
                .into_item();
            write_items(io::stdout(), &[refresh])
        }
    }
}

fn write_items<W>(writer: W, items: &[Item]) -> Result<(), Error>
where
    W: Write,
{
    json::write_items(writer, items)
        .map_err(|e| anyhow!("failed to write alfred items->json: {}", e))
}
