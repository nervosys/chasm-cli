//! Tests for harvest commands
//!
//! Tests for the harvester that collects chat sessions into a database

use rusqlite::Connection;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// Helper Functions
// ============================================================================

#[allow(dead_code)]
fn create_test_harvest_db(dir: &std::path::Path) -> PathBuf {
    let db_path = dir.join("test_harvest.db");

    let conn = Connection::open(&db_path).expect("Failed to create database");

    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            provider TEXT NOT NULL,
            provider_type TEXT,
            workspace_id TEXT,
            workspace_name TEXT,
            title TEXT NOT NULL,
            message_count INTEGER DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            harvested_at INTEGER NOT NULL,
            session_json TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_sessions_provider ON sessions(provider);
        CREATE INDEX IF NOT EXISTS idx_sessions_workspace ON sessions(workspace_id);
        CREATE INDEX IF NOT EXISTS idx_sessions_updated ON sessions(updated_at);
        CREATE INDEX IF NOT EXISTS idx_sessions_title ON sessions(title);

        CREATE TABLE IF NOT EXISTS harvest_metadata (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        INSERT OR REPLACE INTO harvest_metadata (key, value) 
        VALUES ('version', '1.0'),
               ('created_at', datetime('now'));
        "#,
    )
    .expect("Failed to create schema");

    db_path
}

#[allow(dead_code)]
fn insert_test_session(
    conn: &Connection,
    id: &str,
    provider: &str,
    title: &str,
    message_count: i64,
) {
    let now = chrono::Utc::now().timestamp_millis();
    let session_json = format!(r#"{{"sessionId":"{}","title":"{}"}}"#, id, title);

    conn.execute(
        r#"
        INSERT INTO sessions 
        (id, provider, title, message_count, created_at, updated_at, harvested_at, session_json)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        rusqlite::params![
            id,
            provider,
            title,
            message_count,
            now,
            now,
            now,
            session_json
        ],
    )
    .expect("Failed to insert session");
}

// ============================================================================
// Database Schema Tests
// ============================================================================

mod database_schema_tests {
    use super::*;

    #[test]
    fn test_create_harvest_database() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = create_test_harvest_db(temp_dir.path());

        assert!(db_path.exists());

        let conn = Connection::open(&db_path).unwrap();

        // Check sessions table exists
        let table_exists: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sessions'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table_exists, 1);
    }

    #[test]
    fn test_sessions_table_columns() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = create_test_harvest_db(temp_dir.path());
        let conn = Connection::open(&db_path).unwrap();

        // Insert a session to verify columns work
        insert_test_session(&conn, "test-1", "GitHub Copilot", "Test Session", 10);

        // Query it back
        let (id, provider, title, count): (String, String, String, i64) = conn
            .query_row(
                "SELECT id, provider, title, message_count FROM sessions WHERE id = ?",
                ["test-1"],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();

        assert_eq!(id, "test-1");
        assert_eq!(provider, "GitHub Copilot");
        assert_eq!(title, "Test Session");
        assert_eq!(count, 10);
    }

    #[test]
    fn test_metadata_table_exists() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = create_test_harvest_db(temp_dir.path());
        let conn = Connection::open(&db_path).unwrap();

        let version: String = conn
            .query_row(
                "SELECT value FROM harvest_metadata WHERE key = 'version'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(version, "1.0");
    }
}

// ============================================================================
// Session Insert/Update Tests
// ============================================================================

mod session_operations_tests {
    use super::*;

    #[test]
    fn test_insert_session() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = create_test_harvest_db(temp_dir.path());
        let conn = Connection::open(&db_path).unwrap();

        insert_test_session(&conn, "session-1", "Ollama", "Ollama Chat", 5);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_multiple_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = create_test_harvest_db(temp_dir.path());
        let conn = Connection::open(&db_path).unwrap();

        insert_test_session(&conn, "s1", "GitHub Copilot", "Session 1", 10);
        insert_test_session(&conn, "s2", "Cursor", "Session 2", 20);
        insert_test_session(&conn, "s3", "Ollama", "Session 3", 30);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
            .unwrap();

        assert_eq!(count, 3);
    }

    #[test]
    fn test_session_upsert() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = create_test_harvest_db(temp_dir.path());
        let conn = Connection::open(&db_path).unwrap();

        insert_test_session(&conn, "s1", "GitHub Copilot", "Original Title", 10);

        // Insert again with same ID (should update)
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            r#"
            INSERT OR REPLACE INTO sessions 
            (id, provider, title, message_count, created_at, updated_at, harvested_at, session_json)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            rusqlite::params![
                "s1",
                "GitHub Copilot",
                "Updated Title",
                15,
                now,
                now,
                now,
                "{}"
            ],
        )
        .unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);

        let title: String = conn
            .query_row("SELECT title FROM sessions WHERE id = 's1'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(title, "Updated Title");
    }
}

// ============================================================================
// Query Tests
// ============================================================================

mod query_tests {
    use super::*;

    #[test]
    fn test_query_by_provider() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = create_test_harvest_db(temp_dir.path());
        let conn = Connection::open(&db_path).unwrap();

        insert_test_session(&conn, "s1", "GitHub Copilot", "Session 1", 10);
        insert_test_session(&conn, "s2", "GitHub Copilot", "Session 2", 20);
        insert_test_session(&conn, "s3", "Ollama", "Session 3", 30);

        let copilot_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sessions WHERE provider = ?",
                ["GitHub Copilot"],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(copilot_count, 2);
    }

    #[test]
    fn test_query_by_title_search() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = create_test_harvest_db(temp_dir.path());
        let conn = Connection::open(&db_path).unwrap();

        insert_test_session(&conn, "s1", "Copilot", "Rust Development", 10);
        insert_test_session(&conn, "s2", "Copilot", "Python Development", 20);
        insert_test_session(&conn, "s3", "Copilot", "JavaScript Basics", 30);

        let rust_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sessions WHERE LOWER(title) LIKE ?",
                ["%rust%"],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(rust_count, 1);
    }

    #[test]
    fn test_aggregate_message_count() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = create_test_harvest_db(temp_dir.path());
        let conn = Connection::open(&db_path).unwrap();

        insert_test_session(&conn, "s1", "Copilot", "Session 1", 10);
        insert_test_session(&conn, "s2", "Copilot", "Session 2", 20);
        insert_test_session(&conn, "s3", "Copilot", "Session 3", 30);

        let total: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(message_count), 0) FROM sessions",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(total, 60);
    }

    #[test]
    fn test_sessions_by_provider_stats() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = create_test_harvest_db(temp_dir.path());
        let conn = Connection::open(&db_path).unwrap();

        insert_test_session(&conn, "s1", "Copilot", "Session 1", 10);
        insert_test_session(&conn, "s2", "Copilot", "Session 2", 20);
        insert_test_session(&conn, "s3", "Ollama", "Session 3", 30);
        insert_test_session(&conn, "s4", "Cursor", "Session 4", 40);

        let mut stmt = conn
            .prepare(
                "SELECT provider, COUNT(*), SUM(message_count) 
             FROM sessions GROUP BY provider ORDER BY COUNT(*) DESC",
            )
            .unwrap();

        let rows: Vec<(String, i64, i64)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].0, "Copilot");
        assert_eq!(rows[0].1, 2);
    }
}

// ============================================================================
// Index Tests
// ============================================================================

mod index_tests {
    use super::*;

    #[test]
    fn test_provider_index_exists() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = create_test_harvest_db(temp_dir.path());
        let conn = Connection::open(&db_path).unwrap();

        let index_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_sessions_provider'",
            [],
            |row| row.get(0),
        ).unwrap();

        assert_eq!(index_count, 1);
    }

    #[test]
    fn test_all_indexes_exist() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = create_test_harvest_db(temp_dir.path());
        let conn = Connection::open(&db_path).unwrap();

        let expected_indexes = vec![
            "idx_sessions_provider",
            "idx_sessions_workspace",
            "idx_sessions_updated",
            "idx_sessions_title",
        ];

        for idx in expected_indexes {
            let count: i32 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name=?",
                    [idx],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "Index {} should exist", idx);
        }
    }
}

// ============================================================================
// HarvestStats Tests
// ============================================================================

mod stats_tests {
    use chasm::commands::HarvestStats;

    #[test]
    fn test_harvest_stats_default() {
        let stats = HarvestStats::default();

        assert_eq!(stats.providers_scanned, 0);
        assert_eq!(stats.workspaces_scanned, 0);
        assert_eq!(stats.sessions_found, 0);
        assert_eq!(stats.sessions_added, 0);
        assert_eq!(stats.sessions_updated, 0);
        assert_eq!(stats.sessions_skipped, 0);
        assert!(stats.errors.is_empty());
    }

    #[test]
    fn test_harvest_stats_tracking() {
        let mut stats = HarvestStats {
            providers_scanned: 5,
            sessions_found: 100,
            sessions_added: 80,
            sessions_updated: 15,
            sessions_skipped: 5,
            ..Default::default()
        };
        stats.errors.push("Test error".to_string());

        assert_eq!(
            stats.sessions_added + stats.sessions_updated + stats.sessions_skipped,
            stats.sessions_found
        );
        assert_eq!(stats.errors.len(), 1);
    }
}

// ============================================================================
// Export Format Tests
// ============================================================================

mod export_tests {
    use super::*;

    #[test]
    fn test_export_json_format() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = create_test_harvest_db(temp_dir.path());
        let conn = Connection::open(&db_path).unwrap();

        // Insert a session with proper JSON
        let session_json = r#"{"sessionId":"test-1","customTitle":"Test"}"#;
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "INSERT INTO sessions (id, provider, title, message_count, created_at, updated_at, harvested_at, session_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params!["test-1", "Copilot", "Test", 5, now, now, now, session_json],
        ).unwrap();

        // Query the JSON back
        let json: String = conn
            .query_row(
                "SELECT session_json FROM sessions WHERE id = 'test-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        // Verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["sessionId"], "test-1");
    }
}
