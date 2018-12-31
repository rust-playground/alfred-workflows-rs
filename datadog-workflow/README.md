# Datadog Workflow

Datadog Alfred Workflow to cache and search dashboards, screenboards and monitors

Requirements
-------------
sqlite - cache and config values are stored in an sqlite database
Datadog Application & API Key - for Datadog API access

Installation
-------------
1. Download datadog-workflow.alfredworkflow from the repo's [releases](https://github.com/rust-playground/alfred-workflows-rs/releases) section
2. Install in Alfred (double-click)

Setup
------
1. Have your Datadog Application key ready, if you don't have one you can find/generate here `https://{company}.datadoghq.com/account/settings#api`
2. In Alfred type `dd ` you'll be presented with a settings + refresh option, navigate to settings, hit *TAB*
3. You be presented with 2 options, ont for applicatio  key and another for api key, navigate to the application key, hit *TAB*
4. Paste in you key and hit *ENTER*
5. Repeat steps 1->4 for the API key.
6. In Alfred type `dd `, navigate to refresh, hit *ENTER* to cache/index your Datsdog timeboards, screenboards and monitors; this may take some time depending on the number you organization has, there will be a notification popup once complete.

Usage
------
- `dd d [query]...` which queries for timeboards and screenboards together
- `dd t [query]...` which queries for timeboards
- `dd s [query]...` which queries for screenboards
- `dd m [OPTIONS] [query]...` which queries for monitors
  - `--tag <tag>` this options allows you to filter monitors by a singe tag attached to them.

Misc
----
the sqlite database is located at $HOME/.alfred/workflows/datadog/db.sqlite3