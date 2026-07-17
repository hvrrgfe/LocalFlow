use rusqlite::{Connection, Result as SqlResult, params};

/// Version-based SQLite migrations for LocalFlow.
/// Each migration is identified by a version number and can be safely re-run.
pub struct Migrations;

impl Migrations {
    /// Run all pending migrations.
    pub fn run(conn: &Connection) -> SqlResult<()> {
        Self::ensure_migrations_table(conn)?;
        let current_version = Self::current_version(conn)?;

        let migrations = Self::all_migrations();
        for (version, name, sql) in migrations {
            if version > current_version {
                tracing::info!(version, name, "Running migration");
                conn.execute_batch(sql)?;
                conn.execute(
                    "INSERT INTO _migrations (version, name) VALUES (?1, ?2)",
                    params![version, name],
                )?;
                tracing::info!(version, name, "Migration completed");
            }
        }

        Ok(())
    }

    fn ensure_migrations_table(conn: &Connection) -> SqlResult<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS _migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )
    }

    fn current_version(conn: &Connection) -> SqlResult<i64> {
        let result: std::result::Result<i64, _> = conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM _migrations",
            [],
            |row| row.get(0),
        );
        result.or(Ok(0))
    }

    fn all_migrations() -> Vec<(i64, &'static str, &'static str)> {
        vec![
            (1, "initial_schema", Self::v1_initial_schema()),
            (2, "add_audit_logs", Self::v2_audit_logs()),
            (3, "add_indexes", Self::v3_indexes()),
        ]
    }

    fn v1_initial_schema() -> &'static str {
        include_str!("../migrations/001_initial_schema.sql")
    }

    fn v2_audit_logs() -> &'static str {
        include_str!("../migrations/002_audit_logs.sql")
    }

    fn v3_indexes() -> &'static str {
        include_str!("../migrations/003_indexes.sql")
    }
}
