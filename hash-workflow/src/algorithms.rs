use base64::Engine;
use base64::engine::DecodePaddingMode;
use base64::engine::general_purpose::{
    GeneralPurpose, GeneralPurposeConfig, STANDARD, URL_SAFE_NO_PAD,
};
use blake2::Blake2b512;
use md5::Md5;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC};
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha384, Sha512};
use sha3::{Sha3_256, Sha3_512};
use std::iter;

/// RFC 3986 unreserved characters are left as-is; everything else is percent-encoded.
const URL_COMPONENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

const URL_SAFE_ANY_PAD: GeneralPurpose = GeneralPurpose::new(
    &base64::alphabet::URL_SAFE,
    GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

/// Display name plus short aliases accepted by `e/<algo>` / `d/<algo>`.
struct Id {
    name: &'static str,
    aliases: &'static [&'static str],
}

impl Id {
    fn keys(&self) -> impl Iterator<Item = &'static str> {
        iter::once(self.name).chain(self.aliases.iter().copied())
    }

    /// Exact matches ignore separators (`sha256` == `SHA-256`); prefix matches keep
    /// them, so `sha3` matches `SHA3-256` but not `SHA-384`.
    fn matches_exact(&self, query: &str) -> bool {
        let q = compact(query);
        self.keys().any(|k| compact(k) == q)
    }

    fn matches_prefix(&self, query: &str) -> bool {
        let q = dashed(query);
        self.keys().any(|k| dashed(k).starts_with(&q))
    }

    /// Fallback for partially typed names without separators: `sha25` -> `SHA-256`.
    fn matches_compact_prefix(&self, query: &str) -> bool {
        let q = compact(query);
        self.keys().any(|k| compact(k).starts_with(&q))
    }

    fn matches_exact_or_prefix(&self, query: &str) -> bool {
        self.matches_exact(query) || self.matches_prefix(query)
    }
}

type Matcher = fn(&Id, &str) -> bool;

trait Algorithm {
    fn id(&self) -> &Id;
}

struct Encoder {
    id: Id,
    encode: fn(&[u8]) -> String,
}

impl Encoder {
    const fn new(
        name: &'static str,
        aliases: &'static [&'static str],
        encode: fn(&[u8]) -> String,
    ) -> Self {
        Self {
            id: Id { name, aliases },
            encode,
        }
    }
}

impl Algorithm for Encoder {
    fn id(&self) -> &Id {
        &self.id
    }
}

struct Decoder {
    id: Id,
    /// Cheap check that `input` has this encoding's shape, so decoders whose alphabet
    /// merely overlaps the input are never run.
    accepts: fn(&str) -> bool,
    decode: fn(&str) -> Option<Vec<u8>>,
}

impl Decoder {
    const fn new(
        name: &'static str,
        aliases: &'static [&'static str],
        accepts: fn(&str) -> bool,
        decode: fn(&str) -> Option<Vec<u8>>,
    ) -> Self {
        Self {
            id: Id { name, aliases },
            accepts,
            decode,
        }
    }
}

impl Algorithm for Decoder {
    fn id(&self) -> &Id {
        &self.id
    }
}

const BASE64_ALIASES: &[&str] = &["b64"];
const BASE64_URL_ALIASES: &[&str] = &["b64url", "base64url"];
const URL_ALIASES: &[&str] = &["url", "percent"];

/// Encode-mode algorithms in display order.
const ENCODERS: &[Encoder] = &[
    Encoder::new("SHA-256", &[], digest_hex::<Sha256>),
    Encoder::new("SHA-1", &[], digest_hex::<Sha1>),
    Encoder::new("MD5", &[], digest_hex::<Md5>),
    Encoder::new("Base64", BASE64_ALIASES, |b| STANDARD.encode(b)),
    Encoder::new("Base64 URL-safe", BASE64_URL_ALIASES, |b| {
        URL_SAFE_NO_PAD.encode(b)
    }),
    Encoder::new("Hex", &[], |b| hex::encode(b)),
    Encoder::new("URL encoding", URL_ALIASES, |b| {
        percent_encoding::percent_encode(b, URL_COMPONENT).to_string()
    }),
    Encoder::new("SHA-512", &[], digest_hex::<Sha512>),
    Encoder::new("SHA-384", &[], digest_hex::<Sha384>),
    Encoder::new("SHA3-256", &[], digest_hex::<Sha3_256>),
    Encoder::new("SHA3-512", &[], digest_hex::<Sha3_512>),
    Encoder::new(
        "BLAKE2b-512",
        &["blake2", "blake2b"],
        digest_hex::<Blake2b512>,
    ),
    Encoder::new("BLAKE3", &[], |b| blake3::hash(b).to_hex().to_string()),
    Encoder::new("CRC32", &[], |b| format!("{:08x}", crc32fast::hash(b))),
];

/// Decode-mode algorithms in display order.
const DECODERS: &[Decoder] = &[
    Decoder::new(
        "Base64",
        BASE64_ALIASES,
        |s| is_base64(s, b'+', b'/'),
        |s| STANDARD.decode(s).ok(),
    ),
    Decoder::new(
        "Base64 URL-safe",
        BASE64_URL_ALIASES,
        |s| is_base64(s, b'-', b'_'),
        |s| URL_SAFE_ANY_PAD.decode(s).ok(),
    ),
    Decoder::new("Hex", &[], is_hex, |s| hex::decode(s).ok()),
    Decoder::new("URL encoding", URL_ALIASES, has_percent_escape, |s| {
        Some(percent_encoding::percent_decode_str(s).collect())
    }),
];

#[derive(Debug, PartialEq, Eq)]
pub struct Output {
    pub name: &'static str,
    pub value: String,
}

/// An algorithm offered while the user is still typing `e/<prefix>` / `d/<prefix>`.
#[derive(Debug, PartialEq, Eq)]
pub struct Suggestion {
    pub name: &'static str,
    /// Selects exactly this algorithm when used as `<algo>`.
    pub key: String,
}

#[derive(Debug, thiserror::Error)]
#[error("unknown algorithm '{query}'")]
pub struct UnknownAlgorithm {
    pub query: String,
    pub available: Vec<&'static str>,
}

fn digest_hex<D: Digest>(input: &[u8]) -> String {
    hex::encode(D::digest(input))
}

/// Runs every hash and encoding over `input`, or only those matching `algo`.
///
/// # Errors
///
/// Will return `Err` if `algo` matches no encoder.
pub fn encode(input: &[u8], algo: Option<&str>) -> Result<Vec<Output>, UnknownAlgorithm> {
    Ok(select(ENCODERS, algo)?
        .into_iter()
        .map(|e| Output {
            name: e.id.name,
            value: (e.encode)(input),
        })
        .collect())
}

/// Runs decoders over `input`, keeping only results that are printable UTF-8 and
/// weren't already produced by an earlier decoder.
///
/// Without `algo`, only decoders whose shape matches `input` run. Naming an algorithm
/// skips the shape check, so e.g. hex-shaped base64 can still be decoded.
///
/// # Errors
///
/// Will return `Err` if `algo` matches no decoder.
pub fn decode(input: &str, algo: Option<&str>) -> Result<Vec<Output>, UnknownAlgorithm> {
    let explicit = algo.is_some();
    let mut outputs: Vec<Output> = Vec::new();
    for d in select(DECODERS, algo)? {
        if !explicit && !(d.accepts)(input) {
            continue;
        }
        let Some(value) = (d.decode)(input).and_then(|b| String::from_utf8(b).ok()) else {
            continue;
        };
        if !is_printable(&value) || outputs.iter().any(|o| o.value == value) {
            continue;
        }
        outputs.push(Output {
            name: d.id.name,
            value,
        });
    }
    Ok(outputs)
}

/// Encoders whose name or alias matches `prefix`, for autocompletion.
///
/// # Errors
///
/// Will return `Err` if `prefix` matches no encoder.
pub fn encoders_matching(prefix: &str) -> Result<Vec<Suggestion>, UnknownAlgorithm> {
    suggest(ENCODERS, prefix)
}

/// Decoders whose name or alias matches `prefix`, for autocompletion.
///
/// # Errors
///
/// Will return `Err` if `prefix` matches no decoder.
pub fn decoders_matching(prefix: &str) -> Result<Vec<Suggestion>, UnknownAlgorithm> {
    suggest(DECODERS, prefix)
}

/// Unlike [`select`], exact matches don't hide prefix matches, so `b64` still offers
/// Base64 URL-safe.
fn suggest<T: Algorithm>(table: &[T], prefix: &str) -> Result<Vec<Suggestion>, UnknownAlgorithm> {
    let matchers: &[Matcher] = &[Id::matches_exact_or_prefix, Id::matches_compact_prefix];
    Ok(first_matching(table, prefix, matchers)?
        .into_iter()
        .map(|a| Suggestion {
            name: a.id().name,
            key: dashed(a.id().name),
        })
        .collect())
}

/// Picks the algorithms matching `query`, case-insensitively. An exact name/alias
/// match wins (`b64` -> Base64 only); then prefix matches (`sha3` -> SHA3-256,
/// SHA3-512); then prefixes ignoring separators (`sha25` -> SHA-256). No `query`
/// selects everything.
fn select<'a, T: Algorithm>(
    table: &'a [T],
    query: Option<&str>,
) -> Result<Vec<&'a T>, UnknownAlgorithm> {
    let Some(query) = query else {
        return Ok(table.iter().collect());
    };
    let matchers: &[Matcher] = &[
        Id::matches_exact,
        Id::matches_prefix,
        Id::matches_compact_prefix,
    ];
    first_matching(table, query, matchers)
}

/// Entries matched by the first matcher that matches anything, in table order.
fn first_matching<'a, T: Algorithm>(
    table: &'a [T],
    query: &str,
    matchers: &[Matcher],
) -> Result<Vec<&'a T>, UnknownAlgorithm> {
    matchers
        .iter()
        .map(|m| {
            table
                .iter()
                .filter(|a| m(a.id(), query))
                .collect::<Vec<_>>()
        })
        .find(|matched| !matched.is_empty())
        .ok_or_else(|| unknown(table, query))
}

fn unknown<T: Algorithm>(table: &[T], query: &str) -> UnknownAlgorithm {
    UnknownAlgorithm {
        query: query.to_string(),
        available: table.iter().map(|a| a.id().name).collect(),
    }
}

/// Lowercase with separators removed: `SHA3-256` -> `sha3256`.
fn compact(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '-' | '_' | ' '))
        .flat_map(char::to_lowercase)
        .collect()
}

/// Lowercase with every separator as `-`: `Base64 URL-safe` -> `base64-url-safe`.
fn dashed(s: &str) -> String {
    s.chars()
        .map(|c| if matches!(c, '_' | ' ') { '-' } else { c })
        .flat_map(char::to_lowercase)
        .collect()
}

fn is_hex(s: &str) -> bool {
    s.len().is_multiple_of(2) && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Base64 alphabet with the given 62nd/63rd characters and at most two trailing `=`.
/// Hex-shaped input is excluded: it is technically valid base64, but base64 of real
/// text almost always contains a letter outside `a-f`.
fn is_base64(s: &str, c62: u8, c63: u8) -> bool {
    let body = s.trim_end_matches('=');
    s.len() - body.len() <= 2
        && body
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == c62 || b == c63)
        && !is_hex(s)
}

fn has_percent_escape(s: &str) -> bool {
    s.as_bytes()
        .windows(3)
        .any(|w| w[0] == b'%' && w[1].is_ascii_hexdigit() && w[2].is_ascii_hexdigit())
}

/// Rejects decoded "text" containing control characters, which almost always means the
/// input only coincidentally matched a decoder's shape (e.g. base64 `AAAA` -> NULs).
fn is_printable(s: &str) -> bool {
    s.chars()
        .all(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encoded(name: &str, input: &str) -> String {
        encode(input.as_bytes(), None)
            .unwrap()
            .into_iter()
            .find(|o| o.name == name)
            .unwrap()
            .value
    }

    fn decoded_names(input: &str) -> Vec<&'static str> {
        decode(input, None)
            .unwrap()
            .into_iter()
            .map(|o| o.name)
            .collect()
    }

    #[test]
    fn known_hash_vectors() {
        let cases = [
            (
                "SHA-256",
                "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9",
            ),
            ("SHA-1", "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed"),
            ("MD5", "5eb63bbbe01eeed093cb22bb8f5acdc3"),
            ("CRC32", "0d4a1185"),
            (
                "BLAKE3",
                "d74981efa70a0c880b8d8c1985d075dbcbf679b99a5f9914e5aaf96b831a9e24",
            ),
        ];
        for (name, want) in cases {
            assert_eq!(encoded(name, "hello world"), want, "{name}");
        }
    }

    #[test]
    fn known_encoding_vectors() {
        assert_eq!(encoded("Base64", "hello world"), "aGVsbG8gd29ybGQ=");
        assert_eq!(encoded("Base64 URL-safe", "??>>"), "Pz8-Pg");
        assert_eq!(encoded("Hex", "hi"), "6869");
        assert_eq!(
            encoded("URL encoding", "a b&c=d/e-f"),
            "a%20b%26c%3Dd%2Fe-f"
        );
    }

    #[test]
    fn every_encoding_round_trips_through_decode() {
        let input = "hello world?>/&";
        for name in ["Base64", "Base64 URL-safe", "Hex", "URL encoding"] {
            let value = encoded(name, input);
            let out = decode(&value, None).unwrap();
            assert!(
                out.iter().any(|o| o.name == name && o.value == input),
                "{name}: {value} -> {out:?}"
            );
        }
    }

    #[test]
    fn url_safe_base64_decodes_with_or_without_padding() {
        assert_eq!(decoded_names("Pz8-Pg"), ["Base64 URL-safe"]);
        assert_eq!(decoded_names("Pz8-Pg=="), ["Base64 URL-safe"]);
    }

    #[test]
    fn identical_results_are_shown_once() {
        assert_eq!(
            decode("aGVsbG8gd29ybGQ=", None).unwrap(),
            [Output {
                name: "Base64",
                value: "hello world".to_string()
            }]
        );
    }

    #[test]
    fn control_characters_are_rejected() {
        assert_eq!(decoded_names("AAAA"), Vec::<&str>::new());
        assert_eq!(decoded_names("00"), Vec::<&str>::new());
    }

    #[test]
    fn hex_shaped_input_is_not_base64_decoded() {
        // `a000` is valid base64 for the printable bytes `kM4`.
        assert_eq!(decoded_names("a000"), Vec::<&str>::new());
        assert_eq!(decoded_names("6869"), ["Hex"]);
    }

    #[test]
    fn url_decoding_requires_a_percent_escape() {
        assert_eq!(decoded_names("a+b"), Vec::<&str>::new());
        assert_eq!(decoded_names("100%"), Vec::<&str>::new());
        assert_eq!(decoded_names("a%2Bb"), ["URL encoding"]);
    }

    #[test]
    fn multiline_text_is_kept() {
        assert_eq!(decoded_names("YQpi"), ["Base64"]);
    }

    #[test]
    fn undecodable_input_yields_nothing() {
        assert_eq!(decoded_names("not valid!"), Vec::<&str>::new());
    }

    #[test]
    fn non_utf8_decoded_bytes_are_dropped() {
        assert!(!decoded_names("ff00").contains(&"Hex"));
    }

    fn encoded_names(algo: &str) -> Vec<&'static str> {
        encode(b"x", Some(algo))
            .unwrap()
            .into_iter()
            .map(|o| o.name)
            .collect()
    }

    fn forced(input: &str, algo: &str) -> Vec<String> {
        decode(input, Some(algo))
            .unwrap()
            .into_iter()
            .map(|o| o.value)
            .collect()
    }

    #[test]
    fn exact_name_or_alias_selects_one_algorithm() {
        assert_eq!(encoded_names("sha256"), ["SHA-256"]);
        assert_eq!(encoded_names("SHA-256"), ["SHA-256"]);
        assert_eq!(encoded_names("b64"), ["Base64"]);
        assert_eq!(encoded_names("base64"), ["Base64"]);
        assert_eq!(encoded_names("base64url"), ["Base64 URL-safe"]);
        assert_eq!(encoded_names("url"), ["URL encoding"]);
    }

    #[test]
    fn prefix_selects_every_match_in_display_order() {
        assert_eq!(encoded_names("sha3"), ["SHA3-256", "SHA3-512"]);
        assert_eq!(encoded_names("blake"), ["BLAKE2b-512", "BLAKE3"]);
        assert_eq!(
            encoded_names("sha"),
            [
                "SHA-256", "SHA-1", "SHA-512", "SHA-384", "SHA3-256", "SHA3-512"
            ]
        );
    }

    #[test]
    fn prefix_respects_separators() {
        assert_eq!(
            encoded_names("sha-"),
            ["SHA-256", "SHA-1", "SHA-512", "SHA-384"]
        );
        assert_eq!(encoded_names("base64 url"), ["Base64 URL-safe"]);
        assert_eq!(encoded_names("b64u"), ["Base64 URL-safe"]);
    }

    #[test]
    fn partially_typed_names_without_separators_still_match() {
        for partial in ["sha2", "sha25"] {
            assert_eq!(suggested(partial), ["SHA-256"], "{partial}");
            assert_eq!(encoded_names(partial), ["SHA-256"], "{partial}");
        }
        assert_eq!(suggested("base64u"), ["Base64 URL-safe"]);
        assert_eq!(suggested("sha38"), ["SHA-384"]);
    }

    #[test]
    fn empty_algorithm_selects_everything() {
        assert_eq!(encoded_names("").len(), ENCODERS.len());
    }

    #[test]
    fn unknown_algorithm_lists_available_names() {
        let err = encode(b"x", Some("nope")).unwrap_err();
        assert_eq!(err.query, "nope");
        assert!(err.available.contains(&"SHA-256"));

        let err = decode("x", Some("sha256")).unwrap_err();
        assert_eq!(
            err.available,
            ["Base64", "Base64 URL-safe", "Hex", "URL encoding"]
        );
    }

    #[test]
    fn explicit_decoder_skips_shape_check() {
        assert_eq!(decoded_names("a000"), Vec::<&str>::new());
        assert_eq!(forced("a000", "b64"), ["kM4"]);
    }

    #[test]
    fn explicit_decoder_still_rejects_non_text() {
        assert_eq!(forced("AAAA", "b64"), Vec::<String>::new());
        assert_eq!(forced("not hex", "hex"), Vec::<String>::new());
    }

    fn suggested(prefix: &str) -> Vec<&'static str> {
        encoders_matching(prefix)
            .unwrap()
            .into_iter()
            .map(|s| s.name)
            .collect()
    }

    #[test]
    fn suggestions_list_every_prefix_match() {
        assert_eq!(
            suggested("sha"),
            [
                "SHA-256", "SHA-1", "SHA-512", "SHA-384", "SHA3-256", "SHA3-512"
            ]
        );
        assert_eq!(suggested("b64"), ["Base64", "Base64 URL-safe"]);
        assert_eq!(suggested("sha256"), ["SHA-256"]);
        assert_eq!(suggested("sha3"), ["SHA3-256", "SHA3-512"]);
        assert_eq!(suggested("").len(), ENCODERS.len());
        assert!(decoders_matching("sha").is_err());
    }

    fn selected<T: Algorithm>(table: &[T], key: &str) -> Vec<&'static str> {
        select(table, Some(key))
            .unwrap()
            .into_iter()
            .map(|a| a.id().name)
            .collect()
    }

    #[test]
    fn every_suggestion_key_selects_exactly_its_algorithm() {
        for s in encoders_matching("").unwrap() {
            assert_eq!(selected(ENCODERS, &s.key), [s.name], "{}", s.key);
        }
        for s in decoders_matching("").unwrap() {
            assert_eq!(selected(DECODERS, &s.key), [s.name], "{}", s.key);
        }
    }
}
