CREATE TABLE IF NOT EXISTS repositories (
    name_with_owner TEXT NOT NULL PRIMARY KEY,
    name            TEXT NOT NULL,
    url             TEXT NOT NULL,
    pushed_at       DATETIME NOT NULL
);

CREATE TABLE IF NOT EXISTS config (
    key   TEXT NOT NULL PRIMARY KEY,
    value TEXT NOT NULL
);