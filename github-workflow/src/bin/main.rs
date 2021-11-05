use alfred::{json, Item};
use anyhow::{anyhow, Error};
use clap::{
    app_from_crate, crate_authors, crate_description, crate_name, crate_version, AppSettings, Arg,
    SubCommand,
};
use github_workflow_lib::workflow::GithubWorkflow;
use std::borrow::Cow;
use std::io::Write;
use std::{env, io, process::Command};

const SUBCOMMAND_SETTINGS: &str = ">settings";
const SUBCOMMAND_REFRESH: &str = "refresh";
const SUBCOMMAND_OPEN: &str = "open";
const ARG_INPUT: &str = "input";

fn main() -> Result<(), Error> {
    let matches = app_from_crate!("\n")
        .setting(AppSettings::AllowExternalSubcommands)
        .subcommand(
            SubCommand::with_name(SUBCOMMAND_OPEN)
                .about("opens the provided argument (https://)")
                .arg(
                    Arg::with_name(ARG_INPUT)
                        .long(ARG_INPUT)
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
    let mut wf = GithubWorkflow::new(&api_key, &database_url)?;

    match matches.subcommand() {
        (SUBCOMMAND_SETTINGS, Some(m)) => match m.subcommand() {
            (SUBCOMMAND_REFRESH, Some(_)) => {
                wf.refresh_cache()?;
                println!("Successfully Refreshed GitHub cache");
                Ok(())
            }
            _ => Err(anyhow!("No suitable SubCommand found")),
        },
        (SUBCOMMAND_OPEN, Some(m)) => {
            let input = m.value_of(ARG_INPUT).unwrap_or_default();
            if input.starts_with("https://") {
                Command::new("open")
                    .arg(input)
                    .output()
                    .map_err(|e| anyhow!("failed to execute process: {}", e))?;
            }
            Ok(())
        }
        (external, Some(m)) => {
            let query = match m.args.get("") {
                Some(args) => Cow::Owned(
                    args.vals
                        .iter()
                        .map(|s| s.to_string_lossy().into_owned())
                        .collect::<Vec<String>>()
                        .join(" "),
                ),
                None => Cow::Borrowed(external),
            };
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
    json::write_items(writer, &items[..])
        .map_err(|e| anyhow!("failed to write alfred items->json: {}", e))
}
