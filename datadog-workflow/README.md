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
2. In Alfred set the `API_KEY` and `APPLICATION_KEY` environment variables for the workflow. ![Alfred Settings](https://github.com/rust-playground/alfred-workflows-rs/raw/master/datadog-workflow/datadog.png)
3. In Alfred type `dd `, navigate to refresh, hit *ENTER* to cache/index your Datsdog timeboards, screenboards and monitors; this may take some time depending on the number you organization has, there will be a notification popup once complete.

Usage
------
- `dd d [query]...` which queries for timeboards and screenboards together
- `dd t [query]...` which queries for timeboards
- `dd s [query]...` which queries for screenboards
- `dd m [OPTIONS] [query]...` which queries for monitors
  - `--tag <tag>` this options allows you to filter monitors by a singe tag attached to them.
