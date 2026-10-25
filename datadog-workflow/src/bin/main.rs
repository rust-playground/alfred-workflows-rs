use alfred::{json, Item};
use anyhow::{anyhow, Error};
use datadog_workflow_lib::workflow::Workflow;
use std::io::Write;
use std::{env, io};
use clap::{command, Command, Arg};

const SUBCOMMAND_SETTINGS: &str = "settings";
const SUBCOMMAND_REFRESH: &str = "refresh";
const SUBCOMMAND_TIMEBOARDS: &str = "t";
const SUBCOMMAND_SCREENBOARDS: &str = "s";
const SUBCOMMAND_DASHBOARDS: &str = "d";
const SUBCOMMAND_MONITORS: &str = "m";
const SUBCOMMAND_OPEN: &str = "open";
const ARG_INPUT: &str = "input";
const ARG_QUERY: &str = "query";
const ARG_TAG: &str = "tag";

#[allow(clippy::too_many_lines)]
fn main() -> Result<(), Error> {
    let matches = command!()
        .subcommand(
            Command::new(SUBCOMMAND_OPEN)
                .about("opens the provided argument (https://)")
                .arg(
                    Arg::new(ARG_INPUT)
                        .long(ARG_INPUT)
                        .help("the input value to open")
                        .index(1),
                ),
        )
        .subcommand(
            Command::new(SUBCOMMAND_SETTINGS)
                .about("settings to control")
                .subcommand(
                    Command::new(SUBCOMMAND_REFRESH).about("refreshes the cached data"),
                ),
        )
        .subcommand(
            Command::new(SUBCOMMAND_TIMEBOARDS)
                .about("search for timeboards")
                .arg(
                    Arg::new(ARG_QUERY)
                        .long(ARG_QUERY)
                        .help("the title of the timeboard to query")
                        .num_args(1..)
                        .index(1),
                ),
        )
        .subcommand(
            Command::new(SUBCOMMAND_SCREENBOARDS)
                .about("search for screenboards")
                .arg(
                    Arg::new(ARG_QUERY)
                        .long(ARG_QUERY)
                        .help("the title of the screenboard to query")
                        .num_args(1..)
                        .index(1),
                ),
        )
        .subcommand(
            Command::new(SUBCOMMAND_DASHBOARDS)
                .about("search for dashboards(timeboards + screenboards)")
                .arg(
                    Arg::new(ARG_QUERY)
                        .long(ARG_QUERY)
                        .help("the title of the dashboard to query")
                        .num_args(1..)
                        .index(1),
                ),
        )
        .subcommand(
            Command::new(SUBCOMMAND_MONITORS)
                .about("search for monitors")
                .arg(
                    Arg::new(ARG_TAG)
                        .long(ARG_TAG)
                        .help("the tag to filter monitors by")
                )
                .arg(
                    Arg::new(ARG_QUERY)
                        .long(ARG_QUERY)
                        .help("the name of the monitor to query")
                        .num_args(1..)
                        .index(1),
                ),
        )
        .get_matches();

    let api_key = env::var("API_KEY")?;
    let application_key = env::var("APPLICATION_KEY")?;
    let database_url = env::var("DATABASE_URL")?;
    let api_url = env::var("API_URL")?;
    let subdomain = env::var("SUBDOMAIN")?;
    let mut wf = Workflow::new(
        &api_key,
        &application_key,
        &database_url,
        &api_url,
        &subdomain,
    )?;

    match matches.subcommand() {
        Some((SUBCOMMAND_DASHBOARDS, m)) => {
            let query = m
                .get_many::<String>(ARG_QUERY)
                .unwrap_or_default()
                .map(std::string::String::as_str)
                .collect::<Vec<_>>()
                .join(" ");
            let items = wf.query_dashboards(&query)?;
            write_items(io::stdout(), &items)
        }
        Some((SUBCOMMAND_MONITORS, m)) => {
            let query = m
                .get_many::<String>(ARG_QUERY)
                .unwrap_or_default()
                .map(std::string::String::as_str)
                .collect::<Vec<_>>()
                .join(" ");

            let tag = m.get_one::<String>(ARG_TAG).map(std::string::String::as_str);
            let items = wf.query_monitors(&query, tag)?;
            write_items(io::stdout(), &items)
        }
        Some((SUBCOMMAND_TIMEBOARDS, m)) => {
            let query = m
                .get_many::<String>(ARG_QUERY)
                .unwrap_or_default()
                .map(std::string::String::as_str)
                .collect::<Vec<_>>()
                .join(" ");
            let items = wf.query_timeboards(&query)?;
            write_items(io::stdout(), &items)
        }
        Some((SUBCOMMAND_SCREENBOARDS, m)) => {
            let query = m
                .get_many::<String>(ARG_QUERY)
                .unwrap_or_default()
                .map(std::string::String::as_str)
                .collect::<Vec<_>>()
                .join(" ");
            let items = wf.query_screenboards(&query)?;
            write_items(io::stdout(), &items)
        }
        Some((SUBCOMMAND_SETTINGS, m)) => match m.subcommand() {
            Some((SUBCOMMAND_REFRESH, _)) => {
                wf.refresh_cache()?;
                println!("Successfully Refreshed Datadog cache");
                Ok(())
            }
            _ => Err(anyhow!("No suitable SubCommand found")),
        },
        Some((SUBCOMMAND_OPEN, m)) => {
            let input = m.get_one::<String>(ARG_INPUT)
                .map(std::string::String::as_str)
                .unwrap_or_default();

            if input.starts_with("https://") {
                // 2. Use std::process::Command to avoid conflicting with clap::Command
                std::process::Command::new("open")
                    .arg(input)
                    .output()
                    .map_err(|e| anyhow!("failed to execute process: {e}"))?;
            }
            Ok(())
        }
        _ => {
            let refresh = alfred::ItemBuilder::new(SUBCOMMAND_REFRESH)
                .subtitle("Refresh Cache, be patient you will be notified once complete")
                .arg(format!("{SUBCOMMAND_SETTINGS} {SUBCOMMAND_REFRESH}"))
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
        .map_err(|e| anyhow!("failed to write alfred items->json: {e}"))
}
