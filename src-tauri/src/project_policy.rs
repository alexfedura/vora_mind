use anyhow::{anyhow, Result};
use rusqlite::Connection;

pub(crate) fn ensure_project_writable(connection: &Connection, project_id: &str) -> Result<()> {
    let status: String = connection.query_row(
        "SELECT lifecycle_status FROM research_projects WHERE id = ?1",
        [project_id],
        |row| row.get(0),
    )?;

    if status == "archived" {
        return Err(anyhow!(
            "project {project_id} is archived and cannot be modified"
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::ensure_project_writable;

    #[test]
    fn archived_project_is_not_writable() {
        let connection = Connection::open_in_memory().expect("open database");
        connection
            .execute_batch(
                "CREATE TABLE research_projects (
                     id TEXT PRIMARY KEY,
                     lifecycle_status TEXT NOT NULL
                 );
                 INSERT INTO research_projects (id, lifecycle_status)
                 VALUES ('archived', 'archived');",
            )
            .expect("create project");

        assert!(ensure_project_writable(&connection, "archived").is_err());
    }

    #[test]
    fn non_archived_project_remains_writable() {
        let connection = Connection::open_in_memory().expect("open database");
        connection
            .execute_batch(
                "CREATE TABLE research_projects (
                     id TEXT PRIMARY KEY,
                     lifecycle_status TEXT NOT NULL
                 );
                 INSERT INTO research_projects (id, lifecycle_status)
                 VALUES ('active', 'active');",
            )
            .expect("create project");

        assert!(ensure_project_writable(&connection, "active").is_ok());
    }
}
