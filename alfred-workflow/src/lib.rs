//! This contains common abstractions for reuse in multiple workflows
//!
use alfred::{json, Item};
use anyhow::{anyhow, Error};
use rusqlite::Connection;
use std::{fs, io::Write};

/// Opens or creates if not exists an SQLite database.
///
/// # Arguments
/// * `name` - The name of the workflow, which will create a dedicated sub-directory for.vec!
/// * `f` - A lazily evaluated function that is called when the database is first created.
///
/// # Remarks
/// `name` must be unique or it may conflict with other workflows.
///
/// # Examples
///
/// ```
/// use anyhow::Error;
/// use rusqlite::Connection;
/// use rusqlite::NO_PARAMS;
///
/// fn main() -> Result<(), Error> {
///     let conn = alfred_workflow::open_database_or_else("myworkflow", create_tables)?;
///     Ok(())
/// }
///
/// fn create_tables(conn: &Connection) -> Result<(), Error> {
///     conn.execute(
///         "CREATE TABLE IF NOT EXISTS config (
///             key   TEXT NOT NULL PRIMARY KEY,
///             value TEXT NOT NULL
///         );",
///         NO_PARAMS,
///     )?;
///     Ok(())
/// }
/// ```
pub fn open_database_or_else<F>(name: &str, f: F) -> Result<Connection, Error>
where
    F: Fn(&Connection) -> Result<(), Error>,
{
    let conn: Connection;
    let path = dirs::home_dir()
        .ok_or_else(|| anyhow!("Impossible to get your home dir!"))?
        .join(".alfred")
        .join("workflows")
        .join(name);

    let db = path.join("db.sqlite3");
    if !db.exists() {
        fs::create_dir_all(path)?;
        conn = Connection::open(&db)?;
        f(&conn)?
    } else {
        conn = Connection::open(&db)?
    }
    Ok(conn)
}

/// Writes Alfred items to the provided writer.
///
/// # Arguments
/// * `writer` - the writer to writer iterms to.vec!
/// * `items` - the Alfred items to be written.vec!
///
/// # Examples
/// ```
/// use alfred::{json, Item};
/// use std::{io, io::Write};
/// use anyhow::Error;
///
/// fn main() -> Result<(), Error> {
///     let item = alfred::ItemBuilder::new("settings")
///                .subtitle("settings for the workflow")
///                .into_item();
///      alfred_workflow::write_items(io::stdout(), &[item])
/// }
/// ```
pub fn write_items<W>(writer: W, items: &[Item]) -> Result<(), Error>
where
    W: Write,
{
    json::write_items(writer, &items[..])
        .map_err(|e| anyhow!("failed to write alfred items->json: {}", e))
}
