use alfred::{json, Item};
use clap::{
    app_from_crate, crate_authors, crate_description, crate_name, crate_version, Arg, SubCommand,
};
use datadog_workflow_lib::workflow::DatadogWorkflow;
use failure::{format_err, Error};
use std::io::Write;
use std::{env, io, process::Command};

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

fn main() -> Result<(), Error> {
    let matches = app_from_crate!("\n")
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
                    SubCommand::with_name(SUBCOMMAND_REFRESH).help("refreshes the cached data"),
                ),
        )
        .subcommand(
            SubCommand::with_name(SUBCOMMAND_TIMEBOARDS)
                .about("search for timeboards")
                .arg(
                    Arg::with_name(ARG_QUERY)
                        .long(ARG_QUERY)
                        .help("the title of the timeboard to query")
                        .multiple(true)
                        .index(1),
                ),
        )
        .subcommand(
            SubCommand::with_name(SUBCOMMAND_SCREENBOARDS)
                .about("search for screenboards")
                .arg(
                    Arg::with_name(ARG_QUERY)
                        .long(ARG_QUERY)
                        .help("the title of the screenboard to query")
                        .multiple(true)
                        .index(1),
                ),
        )
        .subcommand(
            SubCommand::with_name(SUBCOMMAND_DASHBOARDS)
                .about("search for dashboards(timeboards + screenboards)")
                .arg(
                    Arg::with_name(ARG_QUERY)
                        .long(ARG_QUERY)
                        .help("the title of the dashboard to query")
                        .multiple(true)
                        .index(1),
                ),
        )
        .subcommand(
            SubCommand::with_name(SUBCOMMAND_MONITORS)
                .about("search for monitors")
                .arg(
                    Arg::with_name(ARG_TAG)
                        .long(ARG_TAG)
                        .help("the tag to filter monitors by")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name(ARG_QUERY)
                        .long(ARG_QUERY)
                        .help("the name of the monitor to query")
                        .multiple(true)
                        .index(1),
                ),
        )
        .get_matches();

    let api_key = env::var("API_KEY")?;
    let application_key = env::var("APPLICATION_KEY")?;
    let database_url = env::var("DATABASE_URL")?;
    let mut wf = DatadogWorkflow::new(&api_key, &application_key, &database_url)?;

    match matches.subcommand() {
        (SUBCOMMAND_DASHBOARDS, Some(m)) => {
            let query = m
                .values_of(ARG_QUERY)
                .unwrap_or_default()
                .collect::<Vec<_>>()
                .join(" ");
            let items = wf.query_dashboards(&query)?;
            write_items(io::stdout(), &items)
        }
        (SUBCOMMAND_MONITORS, Some(m)) => {
            let query = m
                .values_of(ARG_QUERY)
                .unwrap_or_default()
                .collect::<Vec<_>>()
                .join(" ");
            let tag = m.value_of(ARG_TAG);
            let items = wf.query_monitors(&query, tag)?;
            write_items(io::stdout(), &items)
        }
        (SUBCOMMAND_TIMEBOARDS, Some(m)) => {
            let query = m
                .values_of(ARG_QUERY)
                .unwrap_or_default()
                .collect::<Vec<_>>()
                .join(" ");
            let items = wf.query_timeboards(&query)?;
            write_items(io::stdout(), &items)
        }
        (SUBCOMMAND_SCREENBOARDS, Some(m)) => {
            let query = m
                .values_of(ARG_QUERY)
                .unwrap_or_default()
                .collect::<Vec<_>>()
                .join(" ");
            let items = wf.query_screenboards(&query)?;
            write_items(io::stdout(), &items)
        }
        (SUBCOMMAND_SETTINGS, Some(m)) => match m.subcommand() {
            (SUBCOMMAND_REFRESH, Some(_)) => {
                wf.refresh_cache()?;
                println!("Successfully Refreshed Datadog cache");
                Ok(())
            }
            _ => Err(format_err!("No suitable SubCommand found")),
        },
        (SUBCOMMAND_OPEN, Some(m)) => {
            let input = m.value_of(ARG_INPUT).unwrap_or_default();
            if input.starts_with("https://") {
                Command::new("open")
                    .arg(input)
                    .output()
                    .map_err(|e| format_err!("failed to execute process: {}", e))?;
            }
            Ok(())
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
        .map_err(|e| format_err!("failed to write alfred items->json: {}", e))
}
