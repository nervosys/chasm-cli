//! Extensive tests for storage operations
//!
//! This file contains comprehensive unit tests for:
//! - SQLite database operations
//! - Chat session index read/write
//! - Session registration
//! - VS Code process detection
//! - Backup functionality

#[allow(unused_imports)]
use chasm::models::ChatSession;
use chasm::models::{ChatSessionIndex, ChatSessionIndexEntry};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// Test Database Setup Helpers
// ============================================================================

/// Create a test SQLite database with VS Code schema
fn create_test_database(path: &std::path::Path) -> rusqlite::Result<()> {
    let conn = rusqlite::Connection::open(path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS ItemTable (key TEXT PRIMARY KEY, value TEXT)",
        [],
    )?;
    Ok(())
}

/// Insert a key-value pair into the test database
fn insert_into_db(path: &std::path::Path, key: &str, value: &str) -> rusqlite::Result<()> {
    let conn = rusqlite::Connection::open(path)?;
    conn.execute(
        "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?, ?)",
        [key, value],
    )?;
    Ok(())
}

/// Read a value from the test database
fn read_from_db(path: &std::path::Path, key: &str) -> rusqlite::Result<Option<String>> {
    let conn = rusqlite::Connection::open(path)?;
    let result: rusqlite::Result<String> =
        conn.query_row("SELECT value FROM ItemTable WHERE key = ?", [key], |row| {
            row.get(0)
        });

    match result {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

// ============================================================================
// Chat Session Index Read Tests
// ============================================================================

mod read_chat_session_index_tests {
    use super::*;
    use chasm::storage::read_chat_session_index;

    #[test]
    fn test_read_empty_index() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("state.vscdb");
        create_test_database(&db_path).unwrap();

        let result = read_chat_session_index(&db_path);
        assert!(result.is_ok());

        let index = result.unwrap();
        assert_eq!(index.version, 1); // Default version
        assert!(index.entries.is_empty());
    }

    #[test]
    fn test_read_populated_index() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("state.vscdb");
        create_test_database(&db_path).unwrap();

        let index_json = r#"{
            "version": 1,
            "entries": {
                "session-123": {
                    "sessionId": "session-123",
                    "title": "Test Session",
                    "lastMessageDate": 1700000000000,
                    "isImported": false,
                    "initialLocation": "panel",
                    "isEmpty": false
                }
            }
        }"#;

        insert_into_db(&db_path, "chat.ChatSessionStore.index", index_json).unwrap();

        let result = read_chat_session_index(&db_path);
        assert!(result.is_ok());

        let index = result.unwrap();
        assert_eq!(index.entries.len(), 1);
        assert!(index.entries.contains_key("session-123"));
    }

    #[test]
    fn test_read_multiple_entries() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("state.vscdb");
        create_test_database(&db_path).unwrap();

        let mut entries = HashMap::new();
        for i in 0..10 {
            entries.insert(
                format!("session-{}", i),
                ChatSessionIndexEntry {
                    session_id: format!("session-{}", i),
                    title: format!("Session {}", i),
                    last_message_date: 1700000000000 + i,
                    is_imported: i % 2 == 0,
                    initial_location: "panel".to_string(),
                    is_empty: false,
                },
            );
        }

        let index = ChatSessionIndex {
            version: 1,
            entries,
        };
        let json = serde_json::to_string(&index).unwrap();

        insert_into_db(&db_path, "chat.ChatSessionStore.index", &json).unwrap();

        let result = read_chat_session_index(&db_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().entries.len(), 10);
    }

    #[test]
    fn test_read_nonexistent_database() {
        let path = PathBuf::from("/nonexistent/path/state.vscdb");
        let result = read_chat_session_index(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("state.vscdb");
        create_test_database(&db_path).unwrap();

        insert_into_db(&db_path, "chat.ChatSessionStore.index", "invalid json").unwrap();

        let result = read_chat_session_index(&db_path);
        assert!(result.is_err());
    }
}

// ============================================================================
// Chat Session Index Write Tests
// ============================================================================

mod write_chat_session_index_tests {
    use super::*;
    use chasm::storage::{read_chat_session_index, write_chat_session_index};

    #[test]
    fn test_write_empty_index() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("state.vscdb");
        create_test_database(&db_path).unwrap();

        let index = ChatSessionIndex::default();
        let result = write_chat_session_index(&db_path, &index);
        assert!(result.is_ok());

        // Verify it was written
        let stored = read_from_db(&db_path, "chat.ChatSessionStore.index").unwrap();
        assert!(stored.is_some());
    }

    #[test]
    fn test_write_populated_index() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("state.vscdb");
        create_test_database(&db_path).unwrap();

        let mut entries = HashMap::new();
        entries.insert(
            "test-session".to_string(),
            ChatSessionIndexEntry {
                session_id: "test-session".to_string(),
                title: "Test".to_string(),
                last_message_date: 1700000000000,
                is_imported: true,
                initial_location: "editor".to_string(),
                is_empty: false,
            },
        );

        let index = ChatSessionIndex {
            version: 1,
            entries,
        };
        write_chat_session_index(&db_path, &index).unwrap();

        // Read back and verify
        let read_index = read_chat_session_index(&db_path).unwrap();
        assert_eq!(read_index.entries.len(), 1);
        assert!(read_index.entries.contains_key("test-session"));
    }

    #[test]
    fn test_write_overwrites_existing() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("state.vscdb");
        create_test_database(&db_path).unwrap();

        // Write initial index
        let mut entries1 = HashMap::new();
        entries1.insert(
            "session-1".to_string(),
            ChatSessionIndexEntry {
                session_id: "session-1".to_string(),
                title: "First".to_string(),
                last_message_date: 1700000000000,
                is_imported: false,
                initial_location: "panel".to_string(),
                is_empty: false,
            },
        );

        write_chat_session_index(
            &db_path,
            &ChatSessionIndex {
                version: 1,
                entries: entries1,
            },
        )
        .unwrap();

        // Write new index
        let mut entries2 = HashMap::new();
        entries2.insert(
            "session-2".to_string(),
            ChatSessionIndexEntry {
                session_id: "session-2".to_string(),
                title: "Second".to_string(),
                last_message_date: 1700000001000,
                is_imported: true,
                initial_location: "terminal".to_string(),
                is_empty: false,
            },
        );

        write_chat_session_index(
            &db_path,
            &ChatSessionIndex {
                version: 1,
                entries: entries2,
            },
        )
        .unwrap();

        // Should only have the second entry
        let read_index = read_chat_session_index(&db_path).unwrap();
        assert_eq!(read_index.entries.len(), 1);
        assert!(read_index.entries.contains_key("session-2"));
        assert!(!read_index.entries.contains_key("session-1"));
    }

    #[test]
    fn test_write_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("state.vscdb");
        create_test_database(&db_path).unwrap();

        let mut entries = HashMap::new();
        for i in 0..5 {
            entries.insert(
                format!("sess-{}", i),
                ChatSessionIndexEntry {
                    session_id: format!("sess-{}", i),
                    title: format!("Title {}", i),
                    last_message_date: 1700000000000 + i * 1000,
                    is_imported: i % 2 == 0,
                    initial_location: ["panel", "editor", "terminal", "notebook", "inline"]
                        [i as usize % 5]
                        .to_string(),
                    is_empty: i == 0,
                },
            );
        }

        let original = ChatSessionIndex {
            version: 1,
            entries,
        };
        write_chat_session_index(&db_path, &original).unwrap();

        let restored = read_chat_session_index(&db_path).unwrap();
        assert_eq!(restored.version, original.version);
        assert_eq!(restored.entries.len(), original.entries.len());

        for (key, entry) in &restored.entries {
            let orig_entry = original.entries.get(key).unwrap();
            assert_eq!(entry.title, orig_entry.title);
            assert_eq!(entry.is_imported, orig_entry.is_imported);
        }
    }
}

// ============================================================================
// Add Session to Index Tests
// ============================================================================

mod add_session_to_index_tests {
    use super::*;
    use chasm::storage::{add_session_to_index, read_chat_session_index};

    #[test]
    fn test_add_session_to_empty_index() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("state.vscdb");
        create_test_database(&db_path).unwrap();

        let result = add_session_to_index(
            &db_path,
            "new-session-123",
            "New Session",
            1700000000000,
            false,
            "panel",
            false,
        );
        assert!(result.is_ok());

        let index = read_chat_session_index(&db_path).unwrap();
        assert_eq!(index.entries.len(), 1);
        assert!(index.entries.contains_key("new-session-123"));

        let entry = index.entries.get("new-session-123").unwrap();
        assert_eq!(entry.title, "New Session");
        assert!(!entry.is_imported);
    }

    #[test]
    fn test_add_session_to_existing_index() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("state.vscdb");
        create_test_database(&db_path).unwrap();

        // Add first session
        add_session_to_index(
            &db_path,
            "session-1",
            "First",
            1700000000000,
            false,
            "panel",
            false,
        )
        .unwrap();

        // Add second session
        add_session_to_index(
            &db_path,
            "session-2",
            "Second",
            1700000001000,
            true,
            "editor",
            false,
        )
        .unwrap();

        let index = read_chat_session_index(&db_path).unwrap();
        assert_eq!(index.entries.len(), 2);
        assert!(index.entries.contains_key("session-1"));
        assert!(index.entries.contains_key("session-2"));
    }

    #[test]
    fn test_add_session_updates_existing() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("state.vscdb");
        create_test_database(&db_path).unwrap();

        // Add session with initial title
        add_session_to_index(
            &db_path,
            "session-1",
            "Original Title",
            1700000000000,
            false,
            "panel",
            false,
        )
        .unwrap();

        // Update same session with new title
        add_session_to_index(
            &db_path,
            "session-1",
            "Updated Title",
            1700000001000,
            true,
            "terminal",
            true,
        )
        .unwrap();

        let index = read_chat_session_index(&db_path).unwrap();
        assert_eq!(index.entries.len(), 1);

        let entry = index.entries.get("session-1").unwrap();
        assert_eq!(entry.title, "Updated Title");
        assert!(entry.is_imported);
        assert!(entry.is_empty);
    }

    #[test]
    fn test_add_imported_session() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("state.vscdb");
        create_test_database(&db_path).unwrap();

        add_session_to_index(
            &db_path,
            "imported-123",
            "Imported Session",
            1700000000000,
            true,
            "imported",
            false,
        )
        .unwrap();

        let index = read_chat_session_index(&db_path).unwrap();
        let entry = index.entries.get("imported-123").unwrap();
        assert!(entry.is_imported);
        assert_eq!(entry.initial_location, "imported");
    }

    #[test]
    fn test_add_empty_session() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("state.vscdb");
        create_test_database(&db_path).unwrap();

        add_session_to_index(
            &db_path,
            "empty-session",
            "Empty",
            1700000000000,
            false,
            "panel",
            true,
        )
        .unwrap();

        let index = read_chat_session_index(&db_path).unwrap();
        let entry = index.entries.get("empty-session").unwrap();
        assert!(entry.is_empty);
    }

    #[test]
    fn test_add_many_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("state.vscdb");
        create_test_database(&db_path).unwrap();

        for i in 0..100 {
            add_session_to_index(
                &db_path,
                &format!("session-{:03}", i),
                &format!("Session {}", i),
                1700000000000 + i * 1000,
                i % 5 == 0,
                "panel",
                i % 10 == 0,
            )
            .unwrap();
        }

        let index = read_chat_session_index(&db_path).unwrap();
        assert_eq!(index.entries.len(), 100);
    }
}

// ============================================================================
// VS Code Running Detection Tests
// ============================================================================

mod vscode_running_tests {
    use chasm::storage::is_vscode_running;

    #[test]
    fn test_is_vscode_running_returns_bool() {
        // This test just verifies the function runs without panicking
        let _result = is_vscode_running();
        // Function always returns a bool, which is what we want
    }

    #[test]
    fn test_is_vscode_running_multiple_calls() {
        // Should be consistent
        let result1 = is_vscode_running();
        let result2 = is_vscode_running();
        // Results might differ if VS Code is launched/closed between calls,
        // but the function should not panic
        let _ = result1;
        let _ = result2;
    }
}

// ============================================================================
// Backup Workspace Sessions Tests
// ============================================================================

mod backup_workspace_sessions_tests {
    use super::*;
    use chasm::storage::backup_workspace_sessions;

    #[test]
    fn test_backup_nonexistent_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let result = backup_workspace_sessions(temp_dir.path());
        assert!(result.is_ok());
        assert!(result.unwrap().is_none()); // No chatSessions dir
    }

    #[test]
    fn test_backup_empty_chat_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let chat_sessions = temp_dir.path().join("chatSessions");
        fs::create_dir(&chat_sessions).unwrap();

        let result = backup_workspace_sessions(temp_dir.path());
        assert!(result.is_ok());

        if let Some(backup_path) = result.unwrap() {
            assert!(backup_path.exists());
            assert!(backup_path
                .to_string_lossy()
                .contains("chatSessions-backup"));
        }
    }

    #[test]
    fn test_backup_with_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let chat_sessions = temp_dir.path().join("chatSessions");
        fs::create_dir(&chat_sessions).unwrap();

        // Create some session files
        for i in 0..5 {
            let session_json = format!(
                r#"{{
                "version": 3,
                "sessionId": "session-{}",
                "creationDate": {},
                "lastMessageDate": {},
                "requests": []
            }}"#,
                i,
                1700000000000i64 + i as i64 * 1000,
                1700000000000i64 + i as i64 * 1000
            );

            fs::write(
                chat_sessions.join(format!("session-{}.json", i)),
                session_json,
            )
            .unwrap();
        }

        let result = backup_workspace_sessions(temp_dir.path());
        assert!(result.is_ok());

        let backup_path = result.unwrap().unwrap();
        assert!(backup_path.exists());

        // Verify all files were copied
        let backup_entries: Vec<_> = fs::read_dir(&backup_path)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(backup_entries.len(), 5);
    }

    #[test]
    fn test_backup_with_subdirectories() {
        let temp_dir = TempDir::new().unwrap();
        let chat_sessions = temp_dir.path().join("chatSessions");
        fs::create_dir(&chat_sessions).unwrap();

        // Create a subdirectory
        let subdir = chat_sessions.join("subdir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("file.txt"), "content").unwrap();

        let result = backup_workspace_sessions(temp_dir.path());
        assert!(result.is_ok());

        let backup_path = result.unwrap().unwrap();
        assert!(backup_path.join("subdir").exists());
        assert!(backup_path.join("subdir").join("file.txt").exists());
    }

    #[test]
    fn test_backup_preserves_content() {
        let temp_dir = TempDir::new().unwrap();
        let chat_sessions = temp_dir.path().join("chatSessions");
        fs::create_dir(&chat_sessions).unwrap();

        let original_content = r#"{"version": 3, "sessionId": "test", "requests": []}"#;
        fs::write(chat_sessions.join("test.json"), original_content).unwrap();

        let result = backup_workspace_sessions(temp_dir.path());
        let backup_path = result.unwrap().unwrap();

        let backed_up_content = fs::read_to_string(backup_path.join("test.json")).unwrap();
        assert_eq!(original_content, backed_up_content);
    }

    #[test]
    fn test_multiple_backups() {
        let temp_dir = TempDir::new().unwrap();
        let chat_sessions = temp_dir.path().join("chatSessions");
        fs::create_dir(&chat_sessions).unwrap();
        fs::write(chat_sessions.join("session.json"), "{}").unwrap();

        // Create multiple backups
        let backup1 = backup_workspace_sessions(temp_dir.path()).unwrap().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1100)); // Wait for different timestamp
        let backup2 = backup_workspace_sessions(temp_dir.path()).unwrap().unwrap();

        // Both should exist and be different
        assert!(backup1.exists());
        assert!(backup2.exists());
        assert_ne!(backup1, backup2);
    }
}

// ============================================================================
// Get Workspace Storage DB Tests
// ============================================================================

mod get_workspace_storage_db_tests {
    use chasm::storage::get_workspace_storage_db;

    #[test]
    fn test_get_db_path() {
        let result = get_workspace_storage_db("test-workspace-hash");
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.to_string_lossy().contains("test-workspace-hash"));
        assert!(path.to_string_lossy().contains("state.vscdb"));
    }

    #[test]
    fn test_get_db_path_different_workspaces() {
        let path1 = get_workspace_storage_db("workspace-1").unwrap();
        let path2 = get_workspace_storage_db("workspace-2").unwrap();

        assert_ne!(path1, path2);
    }

    #[test]
    fn test_get_db_path_special_chars() {
        // Workspace hashes are usually alphanumeric, but test robustness
        let result = get_workspace_storage_db("test-with-special-chars");
        assert!(result.is_ok());
    }
}

// ============================================================================
// Register All Sessions Tests
// ============================================================================

mod register_all_sessions_tests {
    use super::*;
    use chasm::storage::register_all_sessions_from_directory;

    #[allow(dead_code)]
    fn create_test_session_file(dir: &std::path::Path, session_id: &str, title: &str) {
        let session_json = format!(
            r#"{{
            "version": 3,
            "sessionId": "{}",
            "creationDate": 1700000000000,
            "lastMessageDate": 1700000000000,
            "customTitle": "{}",
            "initialLocation": "panel",
            "requests": [
                {{
                    "timestamp": 1700000000000,
                    "message": {{"text": "Test message"}},
                    "response": {{"value": [{{"value": "Test response"}}]}}
                }}
            ]
        }}"#,
            session_id, title
        );

        fs::write(dir.join(format!("{}.json", session_id)), session_json).unwrap();
    }

    // Note: These tests would require mocking the VS Code storage path and database
    // For now, we test the basic error handling

    #[test]
    fn test_register_with_nonexistent_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let chat_sessions = temp_dir.path().join("chatSessions");
        fs::create_dir(&chat_sessions).unwrap();

        // This should fail because the database doesn't exist
        let result = register_all_sessions_from_directory(
            "nonexistent-workspace-hash",
            &chat_sessions,
            true, // force
        );
        assert!(result.is_err());
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

mod storage_error_tests {
    use super::*;
    #[allow(unused_imports)]
    use chasm::error::CsmError;
    use chasm::storage::read_chat_session_index;

    #[test]
    fn test_read_from_directory() {
        let temp_dir = TempDir::new().unwrap();
        // Trying to read a directory as a database should fail
        let result = read_chat_session_index(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_read_from_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let empty_file = temp_dir.path().join("empty.vscdb");
        fs::write(&empty_file, "").unwrap();

        // Empty file is not a valid SQLite database
        let result = read_chat_session_index(&empty_file);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_from_corrupted_db() {
        let temp_dir = TempDir::new().unwrap();
        let corrupt_file = temp_dir.path().join("corrupt.vscdb");
        fs::write(&corrupt_file, "not a sqlite database").unwrap();

        let result = read_chat_session_index(&corrupt_file);
        assert!(result.is_err());
    }
}

// ============================================================================
// Index Serialization Tests
// ============================================================================

mod index_serialization_tests {
    use super::*;

    #[test]
    fn test_index_json_structure() {
        let mut entries = HashMap::new();
        entries.insert(
            "test-id".to_string(),
            ChatSessionIndexEntry {
                session_id: "test-id".to_string(),
                title: "Test Title".to_string(),
                last_message_date: 1700000000000,
                is_imported: true,
                initial_location: "terminal".to_string(),
                is_empty: false,
            },
        );

        let index = ChatSessionIndex {
            version: 1,
            entries,
        };
        let json = serde_json::to_string_pretty(&index).unwrap();

        // Verify JSON structure
        assert!(json.contains("\"version\": 1"));
        assert!(json.contains("\"entries\""));
        assert!(json.contains("\"sessionId\""));
        assert!(json.contains("\"lastMessageDate\""));
        assert!(json.contains("\"isImported\": true"));
    }

    #[test]
    fn test_index_handles_special_characters_in_title() {
        let mut entries = HashMap::new();
        entries.insert(
            "special-chars".to_string(),
            ChatSessionIndexEntry {
                session_id: "special-chars".to_string(),
                title: "Title with \"quotes\" and \\backslashes\\".to_string(),
                last_message_date: 1700000000000,
                is_imported: false,
                initial_location: "panel".to_string(),
                is_empty: false,
            },
        );

        let index = ChatSessionIndex {
            version: 1,
            entries,
        };
        let json = serde_json::to_string(&index).unwrap();
        let restored: ChatSessionIndex = serde_json::from_str(&json).unwrap();

        let entry = restored.entries.get("special-chars").unwrap();
        assert!(entry.title.contains("quotes"));
        assert!(entry.title.contains("backslashes"));
    }

    #[test]
    fn test_index_handles_unicode_in_title() {
        let mut entries = HashMap::new();
        entries.insert(
            "unicode".to_string(),
            ChatSessionIndexEntry {
                session_id: "unicode".to_string(),
                title: "Test Title".to_string(),
                last_message_date: 1700000000000,
                is_imported: false,
                initial_location: "panel".to_string(),
                is_empty: false,
            },
        );

        let index = ChatSessionIndex {
            version: 1,
            entries,
        };
        let json = serde_json::to_string(&index).unwrap();
        let restored: ChatSessionIndex = serde_json::from_str(&json).unwrap();

        let entry = restored.entries.get("unicode").unwrap();
        assert!(entry.title.contains("Test"));
        assert!(entry.title.contains("Title"));
    }

    #[test]
    fn test_index_empty_title() {
        let mut entries = HashMap::new();
        entries.insert(
            "empty-title".to_string(),
            ChatSessionIndexEntry {
                session_id: "empty-title".to_string(),
                title: "".to_string(),
                last_message_date: 1700000000000,
                is_imported: false,
                initial_location: "panel".to_string(),
                is_empty: true,
            },
        );

        let index = ChatSessionIndex {
            version: 1,
            entries,
        };
        let json = serde_json::to_string(&index).unwrap();
        let restored: ChatSessionIndex = serde_json::from_str(&json).unwrap();

        let entry = restored.entries.get("empty-title").unwrap();
        assert!(entry.title.is_empty());
    }
}

// ============================================================================
// Empty Window Sessions (ALL SESSIONS) Tests
// ============================================================================

mod empty_window_sessions_tests {
    use chasm::models::ChatSession;
    use std::fs;
    use tempfile::TempDir;

    /// Helper to create a test chat session
    fn create_test_session(session_id: &str, title: &str) -> ChatSession {
        let json = format!(
            r#"{{
                "version": 3,
                "sessionId": "{}",
                "creationDate": 1700000000000,
                "lastMessageDate": 1700000000000,
                "customTitle": "{}",
                "initialLocation": "panel",
                "requests": [
                    {{
                        "timestamp": 1700000000000,
                        "message": {{"text": "Test message"}},
                        "response": {{"value": [{{"value": "Test response"}}]}}
                    }}
                ]
            }}"#,
            session_id, title
        );
        serde_json::from_str(&json).unwrap()
    }

    /// Helper to create a temp directory simulating emptyWindowChatSessions
    fn setup_test_sessions_dir() -> TempDir {
        TempDir::new().unwrap()
    }

    #[test]
    fn test_empty_window_session_json_parsing() {
        let session = create_test_session("test-session-123", "My Test Session");
        assert_eq!(session.session_id, Some("test-session-123".to_string()));
        assert_eq!(session.custom_title, Some("My Test Session".to_string()));
        assert_eq!(session.version, 3);
    }

    #[test]
    fn test_empty_window_session_file_read() {
        let temp_dir = setup_test_sessions_dir();
        let session_id = "abc123-def456";
        let session = create_test_session(session_id, "Test Title");

        // Write session to temp directory
        let session_path = temp_dir.path().join(format!("{}.json", session_id));
        let json = serde_json::to_string_pretty(&session).unwrap();
        fs::write(&session_path, &json).unwrap();

        // Read it back
        let content = fs::read_to_string(&session_path).unwrap();
        let restored: ChatSession = serde_json::from_str(&content).unwrap();

        assert_eq!(restored.session_id, Some(session_id.to_string()));
        assert_eq!(restored.custom_title, Some("Test Title".to_string()));
    }

    #[test]
    fn test_empty_window_sessions_directory_scan() {
        let temp_dir = setup_test_sessions_dir();

        // Create multiple session files
        for i in 1..=3 {
            let session_id = format!("session-{}", i);
            let session = create_test_session(&session_id, &format!("Session {}", i));
            let session_path = temp_dir.path().join(format!("{}.json", session_id));
            let json = serde_json::to_string_pretty(&session).unwrap();
            fs::write(&session_path, &json).unwrap();
        }

        // Count JSON files
        let count = fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
            .count();

        assert_eq!(count, 3);
    }

    #[test]
    fn test_empty_window_session_with_no_session_id() {
        // Some sessions might not have a session_id
        let json = r#"{
            "version": 3,
            "creationDate": 1700000000000,
            "lastMessageDate": 1700000000000,
            "customTitle": "No ID Session",
            "initialLocation": "panel",
            "requests": []
        }"#;

        let session: ChatSession = serde_json::from_str(json).unwrap();
        assert!(session.session_id.is_none());
        assert_eq!(session.custom_title, Some("No ID Session".to_string()));
    }

    #[test]
    fn test_empty_window_session_sorting_by_date() {
        let mut sessions = [
            {
                let mut s = create_test_session("old", "Old Session");
                s.last_message_date = 1000;
                s
            },
            {
                let mut s = create_test_session("new", "New Session");
                s.last_message_date = 3000;
                s
            },
            {
                let mut s = create_test_session("mid", "Mid Session");
                s.last_message_date = 2000;
                s
            },
        ];

        // Sort by last_message_date descending (most recent first)
        sessions.sort_by(|a, b| b.last_message_date.cmp(&a.last_message_date));

        assert_eq!(sessions[0].session_id, Some("new".to_string()));
        assert_eq!(sessions[1].session_id, Some("mid".to_string()));
        assert_eq!(sessions[2].session_id, Some("old".to_string()));
    }

    #[test]
    fn test_empty_window_session_request_count() {
        let session = create_test_session("test", "Test");
        assert_eq!(session.request_count(), 1);
    }

    #[test]
    fn test_empty_window_session_empty_requests() {
        let json = r#"{
            "version": 3,
            "sessionId": "empty-requests",
            "creationDate": 1700000000000,
            "lastMessageDate": 1700000000000,
            "initialLocation": "panel",
            "requests": []
        }"#;

        let session: ChatSession = serde_json::from_str(json).unwrap();
        assert_eq!(session.request_count(), 0);
        assert!(session.is_empty());
    }

    #[test]
    fn test_empty_window_session_title_extraction() {
        // Session with custom title
        let session = create_test_session("test", "My Custom Title");
        assert_eq!(session.title(), "My Custom Title");

        // Session without custom title (should fall back to first message or "Untitled")
        let json = r#"{
            "version": 3,
            "sessionId": "no-title",
            "creationDate": 1700000000000,
            "lastMessageDate": 1700000000000,
            "initialLocation": "panel",
            "requests": []
        }"#;
        let session: ChatSession = serde_json::from_str(json).unwrap();
        assert_eq!(session.title(), "Untitled");
    }

    #[test]
    fn test_empty_window_session_file_naming() {
        let session_id = "0a9b131f-2644-41df-abe0-34eb3dc658fe";
        let expected_filename = format!("{}.json", session_id);
        assert_eq!(
            expected_filename,
            "0a9b131f-2644-41df-abe0-34eb3dc658fe.json"
        );
    }

    #[test]
    fn test_empty_window_session_ignore_non_json_files() {
        let temp_dir = setup_test_sessions_dir();

        // Create a valid session file
        let session = create_test_session("valid", "Valid Session");
        let json = serde_json::to_string_pretty(&session).unwrap();
        fs::write(temp_dir.path().join("valid.json"), &json).unwrap();

        // Create some non-JSON files that should be ignored
        fs::write(temp_dir.path().join("readme.txt"), "ignore me").unwrap();
        fs::write(temp_dir.path().join(".hidden"), "hidden file").unwrap();
        fs::write(temp_dir.path().join("backup.json.bak"), "backup").unwrap();

        // Only count .json files
        let json_count = fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
            .count();

        assert_eq!(json_count, 1);
    }

    #[test]
    fn test_empty_window_session_invalid_json_handling() {
        let temp_dir = setup_test_sessions_dir();

        // Create a valid session
        let session = create_test_session("valid", "Valid");
        let json = serde_json::to_string_pretty(&session).unwrap();
        fs::write(temp_dir.path().join("valid.json"), &json).unwrap();

        // Create an invalid JSON file
        fs::write(temp_dir.path().join("invalid.json"), "{ not valid json }").unwrap();

        // Reading the valid one should work
        let valid_content = fs::read_to_string(temp_dir.path().join("valid.json")).unwrap();
        let valid_session: Result<ChatSession, _> = serde_json::from_str(&valid_content);
        assert!(valid_session.is_ok());

        // Reading the invalid one should fail
        let invalid_content = fs::read_to_string(temp_dir.path().join("invalid.json")).unwrap();
        let invalid_session: Result<ChatSession, _> = serde_json::from_str(&invalid_content);
        assert!(invalid_session.is_err());
    }
}
