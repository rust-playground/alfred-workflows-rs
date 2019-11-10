# Buildkite Workflow

Buildkite Alfred Workflow to cache and search pipelines

Requirements
-------------
sqlite - cache and config values are stored in an sqlite database
Buildkite API Key - for Buildkite API access with organization + pipeline read permissions

Installation
-------------
1. Download buildkite-workflow.alfredworkflow from the repo's [releases](https://github.com/rust-playground/alfred-workflows-rs/releases) section
2. Install in Alfred (double-click)

Setup
------
1. Have your Buildkite API Key ready, if you don't have one you can find/generate here `https://buildkite.com/user/api-access-tokens`
2. In Alfred set the `API_KEY` environment variable for the workflow. ![Alfred Settings](https://github.com/rust-playground/alfred-workflows-rs/raw/master/buildkite-workflow/buildkite.png)
3. In Alfred type `bk `, navigate to refresh, hit *ENTER* to cache/index your Buildkite pipelines; this may take some time depending on the number your organizations have, there will be a notification popup once complete.

Usage
------
- `bk [query]...` which queries Buildkite pipelines
