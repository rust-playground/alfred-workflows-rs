use clap::{
    app_from_crate, crate_authors, crate_description, crate_name, crate_version, Arg, SubCommand,
};
use datadog_workflow::workflow::DatadogWorkflow;
use failure::{format_err, Error};
use std::{io, process::Command};

const SUBCOMMAND_SETTINGS: &str = "settings";
const SUBCOMMAND_APPLICATION_KEY: &str = "application_key";
const SUBCOMMAND_API_KEY: &str = "api_key";
const SUBCOMMAND_REFRESH: &str = "refresh";
const SUBCOMMAND_TIMEBOARDS: &str = "t";
const SUBCOMMAND_SCREENBOARDS: &str = "s";
const SUBCOMMAND_DASHBOARDS: &str = "d";
const SUBCOMMAND_MONITORS: &str = "m";
const SUBCOMMAND_OPEN: &str = "open";
const ARG_INPUT: &str = "input";
const ARG_KEY: &str = "key";
const ARG_QUERY: &str = "query";
const ARG_TAG: &str = "tag";
const FLAG_SET: &str = "set";

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
                    SubCommand::with_name(SUBCOMMAND_APPLICATION_KEY)
                        .about("set the Datadog application key")
                        .arg(Arg::with_name(ARG_KEY).help("the application key"))
                        .arg(
                            Arg::with_name(FLAG_SET)
                                .long(FLAG_SET)
                                .help("save the application key"),
                        ),
                )
                .subcommand(
                    SubCommand::with_name(SUBCOMMAND_API_KEY)
                        .about("set the Datadog api key")
                        .arg(Arg::with_name(ARG_KEY).help("the api key"))
                        .arg(
                            Arg::with_name(FLAG_SET)
                                .long(FLAG_SET)
                                .help("save the api key"),
                        ),
                )
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

    let mut wf = DatadogWorkflow::create()?;

    match matches.subcommand() {
        (SUBCOMMAND_DASHBOARDS, Some(m)) => {
            let query = m
                .values_of(ARG_QUERY)
                .unwrap_or_default()
                .collect::<Vec<_>>()
                .join(" ");
            let items = wf.query_dashboards(&query)?;
            alfred_workflow::write_items(io::stdout(), &items)
        }
        (SUBCOMMAND_MONITORS, Some(m)) => {
            let query = m
                .values_of(ARG_QUERY)
                .unwrap_or_default()
                .collect::<Vec<_>>()
                .join(" ");
            let tag = m.value_of(ARG_TAG);
            let items = wf.query_monitors(&query, tag)?;
            alfred_workflow::write_items(io::stdout(), &items)
        }
        (SUBCOMMAND_TIMEBOARDS, Some(m)) => {
            let query = m
                .values_of(ARG_QUERY)
                .unwrap_or_default()
                .collect::<Vec<_>>()
                .join(" ");
            let items = wf.query_timeboards(&query)?;
            alfred_workflow::write_items(io::stdout(), &items)
        }
        (SUBCOMMAND_SCREENBOARDS, Some(m)) => {
            let query = m
                .values_of(ARG_QUERY)
                .unwrap_or_default()
                .collect::<Vec<_>>()
                .join(" ");
            let items = wf.query_screenboards(&query)?;
            alfred_workflow::write_items(io::stdout(), &items)
        }
        (SUBCOMMAND_SETTINGS, Some(m)) => match m.subcommand() {
            (SUBCOMMAND_REFRESH, Some(_)) => {
                wf.refresh_cache()?;
                println!("Successfully Refreshed Datadog cache");
                Ok(())
            }
            (SUBCOMMAND_APPLICATION_KEY, Some(m)) => {
                let key = m.value_of(ARG_KEY).unwrap_or_default();
                if m.is_present(FLAG_SET) {
                    wf.set_application_key(&key)?;
                    println!("Successfully set Datadog application key");
                    return Ok(());
                }
                let item = alfred::ItemBuilder::new(format!(
                    "{} {} {}",
                    SUBCOMMAND_SETTINGS, SUBCOMMAND_APPLICATION_KEY, key
                ))
                .subtitle("set Datadog application key")
                .arg(format!(
                    "{} {} {} --{}",
                    SUBCOMMAND_SETTINGS, SUBCOMMAND_APPLICATION_KEY, key, FLAG_SET
                ))
                .into_item();
                alfred_workflow::write_items(io::stdout(), &[item])
            }
            (SUBCOMMAND_API_KEY, Some(m)) => {
                let key = m.value_of(ARG_KEY).unwrap_or_default();
                if m.is_present(FLAG_SET) {
                    wf.set_api_key(&key)?;
                    println!("Successfully set Datadog api key");
                    return Ok(());
                }
                let item = alfred::ItemBuilder::new(format!(
                    "{} {} {}",
                    SUBCOMMAND_SETTINGS, SUBCOMMAND_API_KEY, key
                ))
                .subtitle("set Datadog api key")
                .arg(format!(
                    "{} {} {} --{}",
                    SUBCOMMAND_SETTINGS, SUBCOMMAND_API_KEY, key, FLAG_SET
                ))
                .into_item();
                alfred_workflow::write_items(io::stdout(), &[item])
            }
            _ => {
                let app_key = alfred::ItemBuilder::new(SUBCOMMAND_APPLICATION_KEY)
                    .subtitle("set application key")
                    .autocomplete(format!(
                        " {} {} ",
                        SUBCOMMAND_SETTINGS, SUBCOMMAND_APPLICATION_KEY
                    ))
                    .arg(format!(
                        "{} {}",
                        SUBCOMMAND_SETTINGS, SUBCOMMAND_APPLICATION_KEY
                    ))
                    .into_item();
                let api_key = alfred::ItemBuilder::new(SUBCOMMAND_API_KEY)
                    .subtitle("set api key")
                    .autocomplete(format!(" {} {} ", SUBCOMMAND_SETTINGS, SUBCOMMAND_API_KEY))
                    .arg(format!(
                        "{} {}",
                        SUBCOMMAND_SETTINGS, SUBCOMMAND_APPLICATION_KEY
                    ))
                    .into_item();
                alfred_workflow::write_items(io::stdout(), &[app_key, api_key])
            }
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
            let settings = alfred::ItemBuilder::new(SUBCOMMAND_SETTINGS)
                .subtitle("settings for the workflow")
                .autocomplete(format!(" {} ", SUBCOMMAND_SETTINGS))
                .arg(SUBCOMMAND_SETTINGS)
                .into_item();

            let refresh = alfred::ItemBuilder::new(SUBCOMMAND_REFRESH)
                .subtitle("Refresh Cache, be patient you will be notified once complete")
                .arg(format!("{} {}", SUBCOMMAND_SETTINGS, SUBCOMMAND_REFRESH))
                .into_item();
            alfred_workflow::write_items(io::stdout(), &[settings, refresh])
        }
    }
}
