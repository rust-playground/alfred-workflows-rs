# Hash Workflow

Hash Alfred Workflow to quickly hash, encode and decode strings. Selecting a result copies it to the clipboard.

Requirements
-------------
N/A

Installation
-------------
1. Download hash-workflow.alfredworkflow from the repo's [releases](https://github.com/rust-playground/alfred-workflows-rs/releases) section
2. Install in Alfred (double-click)

Setup (building the workflow manually)
------
1. Script Filter with keyword `hash`, argument optional, input as `argv`, script `./hash-workflow "$1"`. Quoting `"$1"` preserves the input's whitespace exactly. The "with space" option can be on or off: one leading space, the separator after the keyword, is ignored.
2. Connect it to a `Copy to Clipboard` output with `{query}`.

Usage
------
- `hash <string>` hashes and encodes the string. The default mode is encode.
- `hash e <string>` does the same, with encode mode stated explicitly. Use it to hash a string that itself starts with `d ` or `e `, e.g. `hash e d dog` hashes `d dog`.
- `hash d <string>` decodes the string. Only decoders matching the input's shape are tried. Hex means an even number of `0-9a-f` characters, Base64 uses its own alphabet, and URL decoding needs a `%XX` escape. Results are shown only if they decode to printable UTF-8.

- `hash e/<algo> <string>` / `hash d/<algo> <string>` limit the output to one algorithm or a family. `:` also works as the separator (`e:sha256`). Matching ignores case.
  - An exact name or alias picks that one algorithm; separators are optional. Examples: `sha256`, `SHA-256`, `b64`, `base64url`, `url`.
  - Otherwise every algorithm whose name starts with the given prefix is shown. For example, `sha3` gives SHA3-256 and SHA3-512.
  - Naming a decoder skips the shape check, so `hash d/b64 a000` decodes as base64 even though the input looks like hex.
  - Before typing the input, `hash e/<prefix>` or `hash d/<prefix>` lists the matching algorithms. `hash e/` or `hash d/` lists all of them. Pressing ↵ or ⇥ on one autocompletes it, e.g. to `hash e/sha-256 `.
  - `hash e e/sha256 x` hashes the literal string `e/sha256 x`.

Whitespace in the input is never trimmed. The input is hashed exactly as typed.

| Encode | Decode |
| ------ | ------ |
| SHA-256, SHA-1, MD5 | Base64 |
| Base64, Base64 URL-safe (no padding) | Base64 URL-safe (padding optional) |
| Hex | Hex |
| URL percent-encoding | URL percent-decoding |
| SHA-512, SHA-384, SHA3-256, SHA3-512 | |
| BLAKE2b-512, BLAKE3, CRC32 | |
