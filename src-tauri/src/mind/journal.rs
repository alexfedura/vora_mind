use chrono::NaiveDate;
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JournalEntry {
    #[serde(rename = "date")]
    pub date: String,
    pub content: String,
    pub created_at: i64,
    pub updated_at: i64,
}

pub fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS journal_entries (
            entry_date TEXT PRIMARY KEY,
            content TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );",
    )
}

pub fn list_entries(
    conn: &Connection,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<JournalEntry>> {
    if parse_date(start_date).is_none() || parse_date(end_date).is_none() || start_date > end_date {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let mut statement = conn.prepare(
        "SELECT entry_date, content, created_at, updated_at
         FROM journal_entries
         WHERE entry_date BETWEEN ?1 AND ?2
         ORDER BY entry_date",
    )?;
    let rows = statement.query_map(params![start_date, end_date], map_entry)?;
    rows.collect()
}

pub fn save_entry(
    conn: &Connection,
    date: &str,
    content: &str,
    now: i64,
) -> Result<JournalEntry> {
    if parse_date(date).is_none() {
        return Err(rusqlite::Error::InvalidQuery);
    }
    conn.execute(
        "INSERT INTO journal_entries (entry_date, content, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?3)
         ON CONFLICT(entry_date) DO UPDATE SET content = excluded.content, updated_at = excluded.updated_at",
        params![date, content, now],
    )?;
    conn.query_row(
        "SELECT entry_date, content, created_at, updated_at
         FROM journal_entries WHERE entry_date = ?1",
        [date],
        map_entry,
    )
}

fn parse_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .ok()
        .filter(|date| date.format("%Y-%m-%d").to_string() == value)
}

fn map_entry(row: &rusqlite::Row<'_>) -> Result<JournalEntry> {
    Ok(JournalEntry {
        date: row.get(0)?,
        content: row.get(1)?,
        created_at: row.get(2)?,
        updated_at: row.get(3)?,
    })
}

#[cfg(test)]
mod tests {
    use super::{init_schema, list_entries, save_entry};
    use rusqlite::Connection;

    #[test]
    fn journal_entries_upsert_and_list_with_date_scope() {
        let connection = Connection::open_in_memory().expect("open in-memory database");
        init_schema(&connection).expect("initialize Journal schema");
        let long_content = "daily note ".repeat(10_000);

        save_entry(&connection, "2026-10-24", &long_content, 1).expect("save entry");
        save_entry(&connection, "2026-10-25", "Today's note", 2).expect("save next entry");
        let updated = save_entry(&connection, "2026-10-24", "", 3).expect("update entry");

        assert_eq!(updated.content, "");
        assert_eq!(updated.created_at, 1);
        assert_eq!(updated.updated_at, 3);
        let entries = list_entries(&connection, "2026-10-24", "2026-10-24")
            .expect("list entries for one day");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0], updated);
    }

    #[test]
    fn journal_rejects_invalid_dates_and_ranges() {
        let connection = Connection::open_in_memory().expect("open in-memory database");
        init_schema(&connection).expect("initialize Journal schema");

        assert!(save_entry(&connection, "2026-02-30", "invalid", 1).is_err());
        assert!(list_entries(&connection, "2026-10-25", "2026-10-24").is_err());
    }
}
