mod errors;

use alfred::{json, Item};
use anyhow::Error as AnyError;
use chrono::prelude::*;
use chrono::{Local, Utc};
use chrono_tz::Tz;
use clap::Parser;
use errors::Error;
use std::io;
use std::io::Write;
use std::str::FromStr;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[arg(global = true, short, long, default_value = "UTC")]
    tz: Option<String>,

    #[arg()]
    date: Option<Vec<String>>,
}

const NOW_DT: &str = "now";

fn main() -> Result<(), AnyError> {
    let opts = Cli::parse();

    let tz = opts.tz.unwrap(); // safe because there is a default value

    if let Some(mut date_parts) = opts.date {
        let date_str = if date_parts.len() == 1 {
            date_parts.swap_remove(0)
        } else {
            date_parts.join(" ")
        };
        let date_str = date_str.trim();

        if date_str == NOW_DT {
            let now = Utc::now();
            let dt = parse_timezone_and_date(&now, &tz)?;
            write_variations(&dt)?;
        } else {
            let parsed = anydate::parse_utc(date_str)?;
            let dt = parse_timezone_and_date(&parsed, &tz)?;
            write_variations(&dt)?;
        }
    } else {
        let now = alfred::ItemBuilder::new(NOW_DT)
            .subtitle("Common date time formats for the current time in UTC.")
            .autocomplete(format!(" {NOW_DT}"))
            .arg(format!(" {} --tz {NOW_DT}", &tz))
            .into_item();
        write_items(io::stdout(), &[now])?;
    }
    Ok(())
}

#[inline]
fn parse_timezone_and_date(ndt: &DateTime<Utc>, tz: &str) -> Result<DateTime<Tz>, Error> {
    // there isn't a real timezone PST etc.. so doing a common mapping for ease of use.
    let tz = match tz.to_lowercase().as_str() {
        "pst" => "America/Vancouver",
        "cst" => "America/Winnipeg",
        _ => tz,
    };
    Tz::from_str(tz)
        .map_err(Error::Text)
        .map(|tz| ndt.with_timezone(&tz))
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
        dt.timestamp_nanos_opt().unwrap_or_default().to_string(),
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
        .arg(date_string)
        .into_item()
}
