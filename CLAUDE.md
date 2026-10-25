# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```sh
cargo build -p <crate>                      # e.g. github-workflow, hash-workflow
cargo test -p <crate>                       # all tests for one workflow
cargo test -p <crate> <test_name_substring> # single test
cargo fmt --all -- --check                  # `make lint`
make release                                # release builds for aarch64 + x86_64 apple-darwin

# CI (.github/workflows/rust.yml, macOS) fails on any pedantic lint — run this before pushing:
cargo clippy --all-features --all-targets -- -Dclippy::all -Dclippy::pedantic
```

Run a workflow binary directly to inspect the Alfred JSON it emits, e.g.
`./target/debug/hash-workflow "e/sha256 test" | jq .`. The DB-backed workflows need their env vars, e.g. `API_KEY=... DATABASE_URL=/tmp/gh.db cargo run -p github-workflow -- refresh`.

## Architecture

A Cargo workspace (edition 2024, resolver 3). Each `*-workflow` crate is one standalone Alfred workflow binary. Shared dependency versions (`alfred`, `anyhow`, `thiserror`, `rusqlite`, `clap`) live in the root `[workspace.dependencies]`.

Every binary has the same contract with Alfred. Alfred runs it as a Script Filter, passing the user's query as argv. The binary prints Alfred Script Filter JSON to stdout via `alfred::json::write_items`, and the selected item's `arg` is handed to the next Alfred action (open URL, copy to clipboard). The `.alfredworkflow` bundles and `info.plist` files are **not** in the repo; they are assembled by hand, so the Alfred-side wiring is documented only in each crate's README.

There are two shapes of workflow:

- **API-cached (github, buildkite, datadog):** a `lib.rs` + `src/bin/main.rs` split.
  - `main.rs` reads config from Alfred env vars (`API_KEY`, `DATABASE_URL`, …) and dispatches clap subcommands: `refresh` re-fetches from the remote API into SQLite, and `open <url>` shells out to `open`. Any other input is a search query against the local SQLite cache.
  - `workflow.rs` orchestrates, `<service>.rs` / `<service>_api/` is a blocking `reqwest` client that pages through results, and `database/` holds the `rusqlite` schema and queries.
  - Libraries use `thiserror` enums (`errors.rs`, plus `database/errors.rs`). Binaries use `anyhow`.
  - The GitHub workflow's `API_KEY` is a comma-separated list of `token[@ghes-host]` entries, and refresh merges every account's repositories into one cache (deduplicated on `url`).
- **Pure (date-formats, hash):** a single binary with no DB or network; it computes the output items directly from the query.
  - In hash-workflow, the `ENCODERS`/`DECODERS` tables in `src/algorithms.rs` are the extension point.
  - Query syntax: `e`/`d`/`e/<algo>`/`d/<algo>` qualifiers. A lone `e/<prefix>` returns autocomplete suggestions.
  - Input whitespace is significant to hashes. Clap joins argv with single spaces, and one leading space is stripped as the keyword separator.

`alfred-workflow/` is a small shared helper library (`open_database_or_else`, `write_items`). No workflow currently depends on it.

## Alfred gotchas

- `autocomplete` text is inserted directly after the keyword, so prefix it with a space (see `date-formats-workflow/src/main.rs` and `hash-workflow` `suggestion_item`). Otherwise `h` + `e/sha` becomes `he/sha` when the Script Filter's "with space" option is off.
- Items with `valid(false)` autocomplete on ↵ instead of triggering the action. Use them for hints, errors and suggestions.
- The Script Filter reruns on every keystroke, so partial input (e.g. `e/sha25`) must produce sensible results rather than errors.
