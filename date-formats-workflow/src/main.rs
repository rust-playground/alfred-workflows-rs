use alfred::{json, Item};
use chrono::prelude::*;
use chrono_tz::Tz;
use chrono::{Datelike, Timelike, Utc, Local};
use clap::{
    app_from_crate, crate_authors, crate_description, crate_name, crate_version, AppSettings, Arg,
    SubCommand,
};
use failure::_core::num::ParseIntError;
use failure::{format_err, Error};
use std::io;
use std::io::Write;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

const SUBCOMMAND_NOW: &str = "now";
const SUBCOMMAND_PRINT: &str = "print";
const ARG_TIMEZONE: &str = "tz";
const ARG_VALUE: &str = "value";

fn main() -> Result<(), Error> {
    let matches = app_from_crate!("\n")
        .setting(AppSettings::AllowExternalSubcommands)
        .arg(
            Arg::with_name(ARG_TIMEZONE)
                .long(ARG_TIMEZONE)
                .help("the timezone to display the time in eg. America/Vancouver")
                .short("t")
                .global(true)
                .default_value("UTC"),
        )
        .subcommand(
            SubCommand::with_name(SUBCOMMAND_NOW)
                .about("specifies common date formats for the current time in UTC"),
        )
        .subcommand(
            SubCommand::with_name(SUBCOMMAND_PRINT)
                .about("prints the provided value")
                .arg(
                    Arg::with_name(ARG_VALUE)
                        .long(ARG_VALUE)
                        .help("the raw value to print")
                        .multiple(true)
                        .takes_value(true),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        (SUBCOMMAND_NOW, Some(m)) => {
            let tz = m.value_of(ARG_TIMEZONE).unwrap(); // safe because there is a default value
            let now = Utc::now().naive_utc();
            let dt = parse_timezone_and_date(&now, &tz)?;
            write_variations(&dt)?;
            Ok(())
        }
        (SUBCOMMAND_PRINT, Some(m)) => {
            let values = m
                .values_of(ARG_VALUE)
                .unwrap()
                .map(|s| s.to_owned())
                .collect::<Vec<String>>()
                .join(" ");
            print!("{}", values);
            Ok(())
        }
        (date, Some(m)) => {
            let mut tz = m.value_of(ARG_TIMEZONE).unwrap().to_owned(); // safe because there is a default value
            let time = match m.args.get("") {
                Some(args) => {
                    let mut time_args = Vec::new();
                    let mut iter = args.vals.iter().map(|s| s.to_string_lossy().into_owned());
                    'outer: while let Some(arg) = iter.next() {
                        match arg.as_str() {
                            // some funny business to parse the tz because of accepting
                            // arbitrary text before it
                            "-t" | "-tz" => {
                                break 'outer;
                            }
                            _ => time_args.push(arg),
                        }
                    }
                    tz = iter.next().unwrap_or_else(|| tz.to_owned());
                    time_args.into_iter().collect::<Vec<String>>().join(" ")
                }
                None => String::from(""),
            };
            let dt = date.to_owned() + " " + &time;
            let parsed = parse_datetime(&dt.trim())?;
            let dt = parse_timezone_and_date(&parsed, &tz)?;
            write_variations(&dt)?;
            Ok(())
        }
        _ => {
            let now = alfred::ItemBuilder::new(SUBCOMMAND_NOW)
                .subtitle("Common date time formats for the current time in UTC.")
                .autocomplete(format!(" {} ", SUBCOMMAND_NOW))
                .arg(format!("{} --{} UTC", SUBCOMMAND_NOW, ARG_TIMEZONE))
                .into_item();
            write_items(io::stdout(), &[now])
        }
    }
}

const DATE_TIME_PARSE_FORMATS: &[&str] = &["%Y-%m-%d %H:%M:%S %z"];
const UTC_DATE_TIME_PARSE_FORMATS: &[&str] = &["%Y-%m-%d %H:%M:%S", "%a %b %e %T %Y"];
const NAIVE_DATE_PARSE_FORMATS: &[&str] = &["%Y-%m-%d"];

#[inline]
fn parse_timezone_and_date(ndt: &NaiveDateTime, tz: &str) -> Result<DateTime<Tz>, Error> {
    // there isn't a real timezone PST etc.. so doing a common mapping for ease of use.
    let tz = match tz {
        "PST" => "America/Vancouver",
        "CST" => "America/Winnipeg",
        _ => tz,
    };
    match Tz::from_str(tz) {
        Ok(tz) => Ok(tz.from_utc_datetime(ndt)),
        Err(e) => Err(format_err!("{}", e)),
    }
}

struct UnixExtract {
    seconds: i64,
    ns: u32,
}

#[inline]
fn parse_seconds_ns(dt: &str) -> Result<UnixExtract, ParseIntError> {
    let num = dt.parse::<i64>()?;
    let ns = num % 1_000_000_000;
    Ok(UnixExtract {
        seconds: (num - ns) / 1_000_000_000,
        ns: ns as u32,
    })
}

fn parse_datetime(dt: &str) -> Result<NaiveDateTime, Error> {
    // check lengths and try to parse unix timestamps first
    let time = match dt.len() {
        10 => {
            // unix timestamp - seconds
            match dt.parse() {
                Ok(num) => Ok(NaiveDateTime::from_timestamp(num, 0)),
                Err(e) => Err(format_err!("{}", e)),
            }
        }
        13 => {
            // unix timestamp - milliseconds
            match parse_seconds_ns(dt) {
                Ok(u) => Ok(NaiveDateTime::from_timestamp(u.seconds, u.ns)),
                Err(e) => Err(format_err!("{}", e)),
            }
        }
        19 => {
            // unix timestamp - nanoseconds
            match parse_seconds_ns(dt) {
                Ok(u) => Ok(NaiveDateTime::from_timestamp(u.seconds, u.ns)),
                Err(e) => Err(format_err!("{}", e)),
            }
        }
        _ => Err(format_err!("failed to parse DateTime from unix timestamp")),
    };

    // try to unwrap the common date & times
    let time = match time {
        Ok(ndt) => Ok(ndt),
        Err(_) => match dt.parse::<DateTime<Utc>>() {
            Ok(v) => Ok(v.naive_utc()),
            Err(_) => match DateTime::parse_from_rfc3339(&dt) {
                Ok(v) => Ok(v.naive_utc()),
                Err(_) => match DateTime::parse_from_rfc2822(&dt) {
                    Ok(v) => Ok(v.naive_utc()),
                    Err(e) => Err(e),
                },
            },
        },
    };

    let time = match time {
        Ok(ndt) => Ok(ndt),
        Err(_) => {
            let result = DATE_TIME_PARSE_FORMATS
                .iter()
                .map(|fmt| DateTime::parse_from_str(&dt, fmt))
                .find_map(Result::ok);
            match result {
                Some(v) => Ok(v.naive_utc()),
                None => Err(format_err!("failed to parse DateTime")),
            }
        }
    };

    let time = match time {
        Ok(ndt) => Ok(ndt),
        Err(_) => {
            let result = UTC_DATE_TIME_PARSE_FORMATS
                .iter()
                .map(|fmt| Utc.datetime_from_str(&dt, fmt))
                .find_map(Result::ok);
            match result {
                Some(v) => Ok(v.naive_utc()),
                None => Err(format_err!("failed to parse DateTime")),
            }
        }
    };

    let time = match time {
        Ok(ndt) => Ok(ndt),
        Err(_) => {
            let result = NAIVE_DATE_PARSE_FORMATS
                .iter()
                .map(|fmt| NaiveDate::parse_from_str(&dt, fmt))
                .find_map(Result::ok);
            match result {
                Some(v) => Ok(NaiveDateTime::new(
                    v,
                    NaiveTime::from_num_seconds_from_midnight(0, 0),
                )),
                None => Err(format_err!("failed to parse DateTime")),
            }
        }
    }?;
    Ok(time)
}

#[inline]
fn write_items<W>(writer: W, items: &[Item]) -> Result<(), Error>
where
    W: Write,
{
    json::write_items(writer, &items[..])
        .map_err(|e| format_err!("failed to write alfred items->json: {}", e))
}

#[inline]
fn write_variations(dt: &DateTime<Tz>) -> Result<(), Error> {
    let unix_sec = build_item(dt.timestamp().to_string(), "UNIX timestamp - seconds");
    let unix_milli = build_item(
        dt.timestamp_millis().to_string(),
        "UNIX timestamp - milliseconds",
    );
    let unix_nano = build_item(
        dt.timestamp_nanos().to_string(),
        "UNIX timestamp - nanoseconds",
    );
    let rfc_3339 = build_item(
        dt.to_rfc3339_opts(SecondsFormat::Secs, false),
        "rfc_3339 - iso8601 compatible",
    );
    let rfc_2822 = build_item(dt.to_rfc2822(), "rfc_2822");
    let alt = build_item(dt.format("%e %b %Y %H:%M:%S").to_string(), "");

    let diff = Utc::now().signed_duration_since(dt.with_timezone(&Utc));
    let attr = if diff.num_nanoseconds().unwrap() < 0 {
        "ago"
    } else {
        "to go"
    };
    let diff_str = format!(
        "{:?}d, {:?}h, {:?}m, {:?}s {}",
        diff.num_days().abs(),
        diff.num_hours().abs(),
        diff.num_minutes().abs(),
        diff.num_seconds().abs(),
        attr
    );
    let time_since = build_item(
        diff_str,
        "Time since",
    );

    let time_current_tz = build_item(
        dt.with_timezone(&Local).format("%e %b %Y %H:%M:%S").to_string(),
        "Time in local timezone",
    );


    write_items(
        io::stdout(),
        &[unix_sec, unix_milli, unix_nano, alt, time_current_tz, rfc_2822, rfc_3339, time_since],
    )
}

#[inline]
fn build_item(date_string: String, subtitle: &str) -> Item {
    alfred::ItemBuilder::new(date_string.clone())
        .subtitle(subtitle)
        .arg(format!(
            "{} --{} {}",
            SUBCOMMAND_PRINT, ARG_VALUE, date_string
        ))
        .into_item()
}
