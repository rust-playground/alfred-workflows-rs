mod errors;

use alfred::{json, Item};
use anyhow::Error as AnyError;
use chrono::prelude::*;
use chrono::{Local, Utc};
use chrono_tz::Tz;
use clap::{
    app_from_crate, crate_authors, crate_description, crate_name, crate_version, AppSettings, Arg,
    SubCommand,
};
use errors::Error;
use std::io;
use std::io::Write;
use std::str::FromStr;

const SUBCOMMAND_NOW: &str = "now";
const SUBCOMMAND_PRINT: &str = "print";
const ARG_TIMEZONE: &str = "tz";
const ARG_VALUE: &str = "value";

fn main() -> Result<(), AnyError> {
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
            let now = Utc::now();
            let dt = parse_timezone_and_date(&now.naive_utc(), tz)?;
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
                    for arg in &mut iter {
                        match arg.as_str() {
                            // some funny business to parse the tz because of accepting
                            // arbitrary text before it
                            "-t" | "--tz" => break,
                            _ => time_args.push(arg),
                        }
                    }
                    tz = iter.next().unwrap_or_else(|| tz.to_owned());
                    time_args.into_iter().collect::<Vec<String>>().join(" ")
                }
                None => String::from(""),
            };
            let dt = date.to_owned() + " " + &time;

            let parsed = parse_datetime(dt.trim())?;
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
            Ok(write_items(io::stdout(), &[now])?)
        }
    }
}

#[inline]
fn parse_timezone_and_date(ndt: &NaiveDateTime, tz: &str) -> Result<DateTime<Tz>, Error> {
    // there isn't a real timezone PST etc.. so doing a common mapping for ease of use.
    let tz = match tz.to_lowercase().as_str() {
        "pst" => "America/Vancouver",
        "cst" => "America/Winnipeg",
        _ => tz,
    };
    Tz::from_str(tz)
        .map_err(Error::Text)
        .map(|tz| tz.from_utc_datetime(ndt))
}

#[inline]
fn write_items<W>(writer: W, items: &[Item]) -> Result<(), Error>
where
    W: Write,
{
    Ok(json::write_items(writer, items)?)
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
        dt.to_rfc3339_opts(SecondsFormat::Secs, true),
        "rfc_3339 - iso8601 compatible",
    );
    let rfc_3339_nano = build_item(
        dt.to_rfc3339_opts(SecondsFormat::Nanos, true),
        "rfc_3339_nano - iso8601 compatible",
    );
    let rfc_2822 = build_item(dt.to_rfc2822(), "rfc_2822");
    let alt = build_item(dt.format("%e %b %Y %H:%M:%S").to_string(), "");

    let diff = dt.with_timezone(&Utc).signed_duration_since(Utc::now());
    let attr = if diff.num_nanoseconds().unwrap() < 0 {
        "ago"
    } else {
        "to go"
    };
    let decor = if diff.num_nanoseconds().unwrap() < 0 {
        "Time since"
    } else {
        "Time until"
    };
    let diff_str = format!(
        "{:?}d, {:?}h, {:?}m, {:?}s {}",
        diff.num_days().abs(),
        diff.num_hours().abs() % 24,
        diff.num_minutes().abs() % 60,
        diff.num_seconds().abs() % 60,
        attr
    );
    let time_since = build_item(diff_str, decor);

    let time_current_tz = build_item(
        dt.with_timezone(&Local)
            .format("%e %b %Y %H:%M:%S")
            .to_string(),
        "Time in local timezone",
    );

    write_items(
        io::stdout(),
        &[
            unix_sec,
            unix_milli,
            unix_nano,
            alt,
            time_current_tz,
            rfc_2822,
            rfc_3339,
            rfc_3339_nano,
            time_since,
        ],
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

const DATE_TIME_WITH_TIMEZONE_PARSE_FORMATS: &[&str] = &[
    "%Y-%m-%d %H:%M:%S %z",
    "%Y-%m-%d %H:%M:%S%.f%#z",
    "%Y-%m-%d %H:%M:%S%#z",
    "%Y-%m-%d %H:%M%#z",
];
const DATE_TIME_WITHOUT_TIMEZONE_PARSE_FORMATS: &[&str] = &[
    "%Y-%m-%d %H:%M:%S%.f",
    "%Y-%m-%d %H:%M:%S",
    "%Y-%m-%d %H:%M",
    "%a %b %e %T %Y",
    "%Y-%m-%d %I:%M:%S %P",
    "%Y-%m-%d %I:%M %P",
    "%m/%d/%y %H:%M:%S",
    "%m/%d/%y %H:%M",
    "%m/%d/%y %H:%M:%S%.f",
    "%m/%d/%y %I:%M:%S %P",
    "%m/%d/%y %I:%M %P",
    "%m/%d/%Y %H:%M:%S",
    "%m/%d/%Y %H:%M",
    "%m/%d/%Y %H:%M:%S%.f",
    "%m/%d/%Y %I:%M:%S %P",
    "%m/%d/%Y %I:%M %P",
    "%Y/%m/%d %H:%M:%S",
    "%Y/%m/%d %H:%M",
    "%Y/%m/%d %H:%M:%S%.f",
    "%Y/%m/%d %I:%M:%S %P",
    "%Y/%m/%d %I:%M %P",
    "%y%m%d %H:%M:%S",
    "%Y年%m月%d日%H时%M分%S秒",
];

const DATE_TIME_WITHOUT_TIMEZONE_REPLACE_COMMAS_PARSE_FORMATS: &[&str] = &[
    "%B %d %Y %H:%M:%S",
    "%B %d %Y %H:%M",
    "%B %d %Y %I:%M:%S %P",
    "%B %d %Y %I:%M %P",
    "%d %B %Y %H:%M:%S",
    "%d %B %Y %H:%M",
    "%d %B %Y %H:%M:%S%.f",
    "%d %B %Y %I:%M:%S %P",
    "%d %B %Y %I:%M %P",
];
const NAIVE_DATE_PARSE_FORMATS: &[&str] = &[
    "%Y-%m-%d",
    "%Y-%b-%d",
    "%d %B %y",
    "%d %B %Y",
    "%m/%d/%y",
    "%m/%d/%Y",
    "%Y/%m/%d",
    "%m.%d.%y",
    "%m.%d.%Y",
    "%Y.%m.%d",
    "%Y年%m月%d日",
];

const NAIVE_DATE_REPLACE_COMMAS_PARSE_FORMATS: &[&str] = &["%B %d %y", "%B %d %Y"];

#[inline]
pub fn parse_datetime(s: &str) -> Result<NaiveDateTime, Error> {
    parse_unix_timestamp(s)
        .or_else(|_| parse_is08601(s))
        .or_else(|_| parse_rfc2822(s))
        .or_else(|_| parse_naive_datetime(s))
        .or_else(|_| parse_utc_naive_datetime(s))
        .or_else(|_| parse_utc_naive_datetime_replace_commas(s))
        .or_else(|_| parse_naive_dates(s))
        .or_else(|_| parse_naive_dates_replace_commas(s))
}

fn parse_unix_timestamp(s: &str) -> Result<NaiveDateTime, Error> {
    match s.len() {
        10 => {
            // unix timestamp - seconds
            match s.parse::<i64>() {
                Ok(u) => Ok(NaiveDateTime::from_timestamp(u, 0)),
                Err(e) => Err(e.into()),
            }
        }
        13 => {
            // unix timestamp - milliseconds
            match s.parse::<i64>() {
                Ok(u) => Ok(Utc.timestamp_nanos(u * 1_000_000).naive_utc()),
                Err(e) => Err(e.into()),
            }
        }
        19 => {
            // unix timestamp - nanoseconds
            match s.parse::<i64>() {
                Ok(u) => Ok(Utc.timestamp_nanos(u).naive_utc()),
                Err(e) => Err(e.into()),
            }
        }
        _ => Err(Error::UnixTimestamp),
    }
}

fn parse_is08601(s: &str) -> Result<NaiveDateTime, Error> {
    s.parse::<DateTime<Utc>>()
        .map_or_else(|_| Err(Error::ParseDateTime), |utc| Ok(utc.naive_utc()))
}

fn parse_rfc2822(s: &str) -> Result<NaiveDateTime, Error> {
    DateTime::parse_from_rfc2822(s)
        .map_or_else(|_| Err(Error::ParseDateTime), |dt| Ok(dt.naive_utc()))
}

fn parse_naive_datetime(s: &str) -> Result<NaiveDateTime, Error> {
    DATE_TIME_WITH_TIMEZONE_PARSE_FORMATS
        .iter()
        .map(|fmt| DateTime::parse_from_str(s, fmt))
        .find_map(Result::ok)
        .map_or_else(|| Err(Error::ParseDateTime), |dt| Ok(dt.naive_utc()))
}

fn parse_utc_naive_datetime(s: &str) -> Result<NaiveDateTime, Error> {
    DATE_TIME_WITHOUT_TIMEZONE_PARSE_FORMATS
        .iter()
        .map(|fmt| Utc.datetime_from_str(s, fmt))
        .find_map(Result::ok)
        .map_or_else(|| Err(Error::ParseDateTime), |dt| Ok(dt.naive_utc()))
}

fn parse_utc_naive_datetime_replace_commas(s: &str) -> Result<NaiveDateTime, Error> {
    let s = s.replace(", ", " ");
    DATE_TIME_WITHOUT_TIMEZONE_REPLACE_COMMAS_PARSE_FORMATS
        .iter()
        .map(|fmt| Utc.datetime_from_str(&s, fmt))
        .find_map(Result::ok)
        .map_or_else(|| Err(Error::ParseDateTime), |dt| Ok(dt.naive_utc()))
}

fn parse_naive_dates(s: &str) -> Result<NaiveDateTime, Error> {
    NAIVE_DATE_PARSE_FORMATS
        .iter()
        .map(|fmt| NaiveDate::parse_from_str(s, fmt))
        .find_map(Result::ok)
        .map_or_else(
            || Err(Error::ParseDateTime),
            |dt| {
                Ok(NaiveDateTime::new(
                    dt,
                    NaiveTime::from_num_seconds_from_midnight(0, 0),
                ))
            },
        )
}

fn parse_naive_dates_replace_commas(s: &str) -> Result<NaiveDateTime, Error> {
    let s = s.replace(", ", " ");
    NAIVE_DATE_REPLACE_COMMAS_PARSE_FORMATS
        .iter()
        .map(|fmt| NaiveDate::parse_from_str(&s, fmt))
        .find_map(Result::ok)
        .map_or_else(
            || Err(Error::ParseDateTime),
            |dt| {
                Ok(NaiveDateTime::new(
                    dt,
                    NaiveTime::from_num_seconds_from_midnight(0, 0),
                ))
            },
        )
}
