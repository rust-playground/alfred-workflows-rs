use clap::{
    app_from_crate, crate_authors, crate_description, crate_name, crate_version, AppSettings, Arg,
    SubCommand,
};
use failure::{format_err, Error};
use github_workflow::workflow::GithubWorkflow;
use std::{io, process::Command};

const SUBCOMMAND_SETTINGS: &str = ">settings";
const SUBCOMMAND_LOGIN: &str = "login";
const SUBCOMMAND_REFRESH: &str = "refresh";
const SUBCOMMAND_OPEN: &str = "open";
const ARG_INPUT: &str = "input";
const ARG_TOKEN: &str = "token";
const FLAG_SET: &str = "set";

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
                    SubCommand::with_name(SUBCOMMAND_LOGIN)
                        .arg(Arg::with_name(ARG_TOKEN).help("sets the login token"))
                        .arg(
                            Arg::with_name(FLAG_SET)
                                .long(FLAG_SET)
                                .help("set the login token"),
                        ),
                )
                .subcommand(
                    SubCommand::with_name(SUBCOMMAND_REFRESH).about("refreshes the cached data"),
                ),
        )
        .get_matches();

    let mut wf = GithubWorkflow::create()?;

    match matches.subcommand() {
        (SUBCOMMAND_SETTINGS, Some(m)) => match m.subcommand() {
            (SUBCOMMAND_REFRESH, Some(_)) => {
                wf.refresh_cache()?;
                println!("Successfully Refreshed GitHub cache");
                Ok(())
            }
            (SUBCOMMAND_LOGIN, Some(m)) => {
                let token = m.value_of(ARG_TOKEN).unwrap_or_default();
                if m.is_present(FLAG_SET) {
                    wf.set_token(&token)?;
                    println!("Successfully set GitHub token");
                    return Ok(());
                }
                let item = alfred::ItemBuilder::new(format!(
                    "{} {} {}",
                    SUBCOMMAND_SETTINGS, SUBCOMMAND_LOGIN, token
                ))
                .subtitle("set access token")
                .arg(format!(
                    "{} {} {} --{}",
                    SUBCOMMAND_SETTINGS, SUBCOMMAND_LOGIN, token, FLAG_SET
                ))
                .into_item();
                alfred_workflow::write_items(io::stdout(), &[item])
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
        (external, Some(_)) => {
            let items = wf.query(&external)?;
            alfred_workflow::write_items(io::stdout(), &items)
        }
        _ => {
            let login = alfred::ItemBuilder::new(SUBCOMMAND_LOGIN)
                .subtitle("set access token")
                .autocomplete(format!(" {} {} ", SUBCOMMAND_SETTINGS, SUBCOMMAND_LOGIN))
                .arg(format!("{} {}", SUBCOMMAND_SETTINGS, SUBCOMMAND_LOGIN))
                .into_item();

            let refresh = alfred::ItemBuilder::new(SUBCOMMAND_REFRESH)
                .subtitle("Refresh Cache, be patient you will be notified once complete")
                .arg(format!("{} {}", SUBCOMMAND_SETTINGS, SUBCOMMAND_REFRESH))
                .into_item();
            alfred_workflow::write_items(io::stdout(), &[login, refresh])
        }
    }
}
