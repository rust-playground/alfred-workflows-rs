# Github Workflow

Github Alfred Workflow to cache and search repositories

Requirements
-------------
sqlite - cache and config values are stored in an sqlite database
Github Access Token - for Github API access

Installation
-------------
1. Download github-workflow.alfredworkflow from the repo's [releases](https://github.com/rust-playground/alfred-workflows-rs/releases) section
2. Install in Alfred (double-click)

Setup
------
1. Have your GitHub Access Token ready, if you don't have one you can generate here https://github.com/settings/tokens; you may have to ensure it is authorized for SSO.
2. In Alfred type `gh ` you'll be presented with a login + refresh option navigate to login, hit *TAB*, paste your token in and hit enter
3. In Alfred type `gh `, navigate to refresh, hit *ENTER* to cache/index your GitHub repositories. This may take some time depending on the number of organizations and repositories you have access to, there will be a notification popup once complete.

Usage
------
- `gh <reponame>` which queries Github repositories

Misc
----
the sqlite database is located at $HOME/.alfred/workflows/github/db.sqlite3