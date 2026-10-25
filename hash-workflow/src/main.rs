mod algorithms;

use alfred::{Item, json};
use algorithms::{Output, Suggestion, UnknownAlgorithm};
use anyhow::{Error, anyhow};
use clap::Parser;
use std::io;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Optionally prefixed with `e ` (encode, the default) or `d ` (decode), each of
    /// which may name an algorithm: `e/sha256 `, `d/b64 `. Multiple arguments are
    /// joined with a single space; quote the input to preserve exact whitespace.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    input: Vec<String>,
}

impl Cli {
    fn raw_input(&self) -> String {
        self.input.join(" ")
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Mode {
    Encode,
    Decode,
}

impl Mode {
    const fn qualifier(self) -> &'static str {
        match self {
            Mode::Encode => "e",
            Mode::Decode => "d",
        }
    }

    const fn verb(self) -> &'static str {
        match self {
            Mode::Encode => "Encode",
            Mode::Decode => "Decode",
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Query<'a> {
    /// Run `mode` over `input`, optionally limited to `algo`.
    Run {
        mode: Mode,
        algo: Option<&'a str>,
        input: &'a str,
    },
    /// Only an `e/<prefix>` / `d/<prefix>` qualifier so far; offer matching algorithms.
    Suggest { mode: Mode, prefix: &'a str },
}

const DEFAULT_DECODE_FAILURE: &str = "Input could not be decoded as base64, hex or URL encoding";

fn main() -> Result<(), Error> {
    let opts = Cli::parse();

    let raw = opts.raw_input();
    let raw = strip_keyword_separator(&raw);
    if raw.is_empty() {
        return write_items(&[invalid_item(
            "Type a string to hash or encode",
            "Prefix 'd ' to decode; 'e/<algo> ' or 'd/<algo> ' to pick an algorithm",
        )]);
    }

    let items = match parse_query(raw) {
        Query::Run { mode, algo, input } => run(mode, algo, input),
        Query::Suggest { mode, prefix } => suggest(mode, prefix),
    };
    write_items(&items)
}

/// Alfred passes the space typed after the keyword as part of the query when the
/// Script Filter's "with space" option is off, and autocomplete text starts with one.
fn strip_keyword_separator(raw: &str) -> &str {
    raw.strip_prefix(' ').unwrap_or(raw)
}

/// Splits an optional leading qualifier (`e`, `d`, `e/<algo>` or `d/<algo>`, followed
/// by a single space) off the input, preserving the rest byte-for-byte. A lone
/// `e/<algo>`/`d/<algo>` asks for suggestions. Anything else is encoded literally with
/// every algorithm.
fn parse_query(raw: &str) -> Query<'_> {
    let literal = Query::Run {
        mode: Mode::Encode,
        algo: None,
        input: raw,
    };
    let (head, input) = raw.split_once(' ').unwrap_or((raw, ""));
    let (mode, algo) = match head.split_once(['/', ':']) {
        Some((mode, algo)) => (mode, Some(algo)),
        None => (head, None),
    };
    let mode = match mode {
        "e" => Mode::Encode,
        "d" => Mode::Decode,
        _ => return literal,
    };
    match (algo, input.is_empty()) {
        (Some(prefix), true) => Query::Suggest { mode, prefix },
        (None, true) => literal,
        (algo, false) => Query::Run { mode, algo, input },
    }
}

fn run(mode: Mode, algo: Option<&str>, input: &str) -> Vec<Item<'static>> {
    let result = match mode {
        Mode::Encode => algorithms::encode(input.as_bytes(), algo),
        Mode::Decode => algorithms::decode(input, algo),
    };
    let outputs = match result {
        Ok(outputs) => outputs,
        Err(e) => return vec![unknown_item(&e)],
    };
    if outputs.is_empty() {
        let subtitle = algo.map_or_else(
            || DEFAULT_DECODE_FAILURE.to_string(),
            |a| format!("Input is not valid {a} text"),
        );
        return vec![invalid_item("Nothing to decode", &subtitle)];
    }
    outputs.into_iter().map(build_item).collect()
}

fn suggest(mode: Mode, prefix: &str) -> Vec<Item<'static>> {
    let result = match mode {
        Mode::Encode => algorithms::encoders_matching(prefix),
        Mode::Decode => algorithms::decoders_matching(prefix),
    };
    match result {
        Ok(suggestions) => suggestions
            .into_iter()
            .map(|s| suggestion_item(mode, &s))
            .collect(),
        Err(e) => vec![unknown_item(&e)],
    }
}

/// Not actionable: selecting it autocompletes `e/<key> ` so the user can type the input.
/// The leading space separates it from the keyword when the Script Filter's "with
/// space" option is off; [`strip_keyword_separator`] drops it again.
fn suggestion_item(mode: Mode, s: &Suggestion) -> Item<'static> {
    alfred::ItemBuilder::new(s.name)
        .subtitle(format!("{} with {}", mode.verb(), s.name))
        .autocomplete(format!(" {}/{} ", mode.qualifier(), s.key))
        .valid(false)
        .into_item()
}

fn unknown_item(e: &UnknownAlgorithm) -> Item<'static> {
    invalid_item(
        &format!("Unknown algorithm '{}'", e.query),
        &format!("Available: {}", e.available.join(", ")),
    )
}

fn invalid_item(title: &str, subtitle: &str) -> Item<'static> {
    alfred::ItemBuilder::new(title.to_string())
        .subtitle(subtitle.to_string())
        .valid(false)
        .into_item()
}

fn build_item(o: Output) -> Item<'static> {
    alfred::ItemBuilder::new(o.value.clone())
        .subtitle(o.name)
        .arg(o.value.clone())
        .text_copy(o.value)
        .into_item()
}

fn write_items(items: &[Item]) -> Result<(), Error> {
    json::write_items(io::stdout(), items)
        .map_err(|e| anyhow!("failed to write alfred items->json: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::iter;

    fn q(mode: Mode, algo: Option<&'static str>, input: &'static str) -> Query<'static> {
        Query::Run { mode, algo, input }
    }

    #[test]
    fn qualifier_selects_mode() {
        assert_eq!(parse_query("e x"), q(Mode::Encode, None, "x"));
        assert_eq!(parse_query("d x"), q(Mode::Decode, None, "x"));
    }

    #[test]
    fn qualifier_may_name_an_algorithm() {
        assert_eq!(
            parse_query("e/sha256 foo bar"),
            q(Mode::Encode, Some("sha256"), "foo bar")
        );
        assert_eq!(
            parse_query("d/b64 a000"),
            q(Mode::Decode, Some("b64"), "a000")
        );
    }

    #[test]
    fn colon_is_accepted_as_an_alternative_separator() {
        assert_eq!(
            parse_query("e:sha256 foo"),
            q(Mode::Encode, Some("sha256"), "foo")
        );
        assert_eq!(
            parse_query("d:b64"),
            Query::Suggest {
                mode: Mode::Decode,
                prefix: "b64"
            }
        );
    }

    #[test]
    fn lone_algorithm_qualifier_asks_for_suggestions() {
        let suggest = |mode, prefix| Query::Suggest { mode, prefix };
        assert_eq!(parse_query("e/sha"), suggest(Mode::Encode, "sha"));
        assert_eq!(parse_query("e/sha-256 "), suggest(Mode::Encode, "sha-256"));
        assert_eq!(parse_query("d/"), suggest(Mode::Decode, ""));
    }

    #[test]
    fn no_qualifier_defaults_to_encode_everything() {
        assert_eq!(parse_query("dog"), q(Mode::Encode, None, "dog"));
        assert_eq!(parse_query("d"), q(Mode::Encode, None, "d"));
        assert_eq!(parse_query("d "), q(Mode::Encode, None, "d "));
        assert_eq!(parse_query("x:y z"), q(Mode::Encode, None, "x:y z"));
    }

    #[test]
    fn plain_qualifier_keeps_the_rest_literal() {
        assert_eq!(parse_query("e d dog"), q(Mode::Encode, None, "d dog"));
        assert_eq!(parse_query("e e/sha"), q(Mode::Encode, None, "e/sha"));
    }

    fn raw_input(args: &[&str]) -> String {
        Cli::try_parse_from(iter::once("hash-workflow").chain(args.iter().copied()))
            .unwrap()
            .raw_input()
    }

    #[test]
    fn unquoted_shell_arguments_are_joined() {
        assert_eq!(raw_input(&["e/sha256", "test"]), "e/sha256 test");
        assert_eq!(raw_input(&["a  b "]), "a  b ");
        assert_eq!(raw_input(&[]), "");
    }

    #[test]
    fn input_may_start_with_a_hyphen() {
        assert_eq!(raw_input(&["-foo"]), "-foo");
        assert_eq!(raw_input(&["e", "--bar"]), "e --bar");
    }

    #[test]
    fn one_keyword_separator_space_is_dropped() {
        assert_eq!(strip_keyword_separator(" e/sha-256 "), "e/sha-256 ");
        assert_eq!(strip_keyword_separator("  foo"), " foo");
        assert_eq!(strip_keyword_separator("foo"), "foo");
        assert_eq!(strip_keyword_separator(" "), "");
    }

    #[test]
    fn whitespace_is_preserved() {
        assert_eq!(parse_query("e  a  b "), q(Mode::Encode, None, " a  b "));
        assert_eq!(parse_query(" a\tb "), q(Mode::Encode, None, " a\tb "));
    }
}
