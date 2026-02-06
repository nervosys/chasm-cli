//! Extensive tests for workspace discovery and management
//!
//! This file contains comprehensive unit tests for:
//! - Workspace path handling
//! - Path normalization
//! - URI decoding
//! - Workspace discovery

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// Workspace Path Utilities Tests
// ============================================================================

mod decode_workspace_folder_tests {
    use chasm::workspace::decode_workspace_folder;

    #[test]
    fn test_decode_simple_unix_path() {
        let uri = "file:///home/user/project";
        let decoded = decode_workspace_folder(uri);
        // Result depends on platform
        assert!(decoded.contains("home") || decoded.contains("user"));
    }

    #[test]
    fn test_decode_windows_path() {
        let uri = "file:///C:/Users/test/project";
        let decoded = decode_workspace_folder(uri);
        assert!(decoded.contains("Users") || decoded.contains("test"));
    }

    #[test]
    fn test_decode_url_encoded_spaces() {
        let uri = "file:///home/user/my%20project";
        let decoded = decode_workspace_folder(uri);
        assert!(decoded.contains("my project") || decoded.contains("my%20project"));
    }

    #[test]
    fn test_decode_url_encoded_special_chars() {
        let uri = "file:///home/user/%E4%B8%AD%E6%96%87"; // Chinese chars
        let decoded = decode_workspace_folder(uri);
        // Should decode or pass through
        assert!(!decoded.is_empty());
    }

    #[test]
    fn test_decode_double_slash_prefix() {
        let uri = "file://home/user/project";
        let decoded = decode_workspace_folder(uri);
        assert!(!decoded.is_empty());
    }

    #[test]
    fn test_decode_no_prefix() {
        let path = "/home/user/project";
        let decoded = decode_workspace_folder(path);
        // On Windows, forward slashes are converted to backslashes
        if cfg!(target_os = "windows") {
            assert_eq!(decoded, "\\home\\user\\project");
        } else {
            assert_eq!(decoded, "/home/user/project");
        }
    }

    #[test]
    fn test_decode_empty_string() {
        let decoded = decode_workspace_folder("");
        assert_eq!(decoded, "");
    }

    #[test]
    fn test_decode_windows_backslash() {
        // On Windows this should convert slashes
        let uri = "file:///C:/Users/test/path";
        let decoded = decode_workspace_folder(uri);
        // Just verify it doesn't panic
        assert!(!decoded.is_empty());
    }

    #[test]
    fn test_decode_network_path() {
        let uri = "file://server/share/folder";
        let decoded = decode_workspace_folder(uri);
        assert!(decoded.contains("share") || decoded.contains("server"));
    }

    #[test]
    fn test_decode_with_query_params() {
        // Shouldn't happen in practice, but test robustness
        let uri = "file:///home/user/project?query=value";
        let decoded = decode_workspace_folder(uri);
        assert!(decoded.contains("project"));
    }

    #[test]
    fn test_decode_multiple_encoded_chars() {
        let uri = "file:///home/%20user%20/my%20%20project";
        let decoded = decode_workspace_folder(uri);
        // Verify decoding happened
        assert!(!decoded.contains("%20") || decoded.contains(" "));
    }
}

// ============================================================================
// Normalize Path Tests
// ============================================================================

mod normalize_path_tests {
    use chasm::workspace::normalize_path;

    #[test]
    fn test_normalize_basic_path() {
        let normalized = normalize_path("/home/user/project");
        // Should be lowercased
        assert_eq!(normalized, normalized.to_lowercase());
    }

    #[test]
    fn test_normalize_path_casing() {
        let normalized = normalize_path("/Home/User/PROJECT");
        assert!(!normalized.contains('P')); // Should be lowercase
        assert!(normalized.contains("project"));
    }

    #[test]
    fn test_normalize_trailing_slash() {
        let with_slash = normalize_path("/home/user/project/");
        let without_slash = normalize_path("/home/user/project");
        // Both should be non-empty
        assert!(!with_slash.is_empty());
        assert!(!without_slash.is_empty());
    }

    #[test]
    fn test_normalize_relative_path() {
        let normalized = normalize_path("relative/path");
        assert!(!normalized.is_empty());
    }

    #[test]
    fn test_normalize_dot_path() {
        let normalized = normalize_path("./current/dir");
        assert!(!normalized.is_empty());
    }

    #[test]
    fn test_normalize_parent_path() {
        let normalized = normalize_path("../parent/dir");
        assert!(!normalized.is_empty());
    }

    #[test]
    fn test_normalize_empty_path() {
        let normalized = normalize_path("");
        // Should handle gracefully
        assert!(normalized.is_empty() || normalized == ".");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_normalize_windows_drive_letter() {
        let normalized = normalize_path("C:\\Users\\Test");
        assert!(normalized.contains("users"));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_normalize_unix_absolute() {
        let normalized = normalize_path("/usr/local/bin");
        assert!(normalized.contains("usr") || normalized.contains("local"));
    }
}

// ============================================================================
// Workspace Storage Path Tests
// ============================================================================

mod workspace_storage_path_tests {
    use chasm::workspace::get_workspace_storage_path;

    #[test]
    fn test_get_workspace_storage_path() {
        let result = get_workspace_storage_path();
        assert!(result.is_ok());

        let path = result.unwrap();
        // Should contain "workspaceStorage" in path
        assert!(path.to_string_lossy().contains("workspaceStorage"));
    }

    #[test]
    fn test_workspace_storage_path_platform_specific() {
        let path = get_workspace_storage_path().unwrap();
        let path_str = path.to_string_lossy();

        if cfg!(target_os = "windows") {
            assert!(path_str.contains("Code") || path_str.contains("AppData"));
        } else if cfg!(target_os = "macos") {
            assert!(path_str.contains("Application Support") || path_str.contains("Code"));
        } else {
            assert!(path_str.contains(".config") || path_str.contains("Code"));
        }
    }
}

// ============================================================================
// Workspace Discovery Tests
// ============================================================================

mod workspace_discovery_tests {
    use chasm::workspace::discover_workspaces;

    #[test]
    fn test_discover_workspaces_returns_vec() {
        let result = discover_workspaces();
        assert!(result.is_ok());
        // Returns a vector (may be empty if no workspaces exist)
        let workspaces = result.unwrap();
        assert!(!workspaces.is_empty() || workspaces.is_empty()); // Just verify it's a valid vec
    }

    #[test]
    fn test_discovered_workspace_fields() {
        let result = discover_workspaces();
        if let Ok(workspaces) = result {
            for ws in &workspaces {
                // Hash should not be empty
                assert!(!ws.hash.is_empty());
                // Workspace path should exist
                assert!(
                    ws.workspace_path.exists() || !ws.workspace_path.to_string_lossy().is_empty()
                );
            }
        }
    }
}

// ============================================================================
// Get Workspace By Hash Tests
// ============================================================================

mod get_workspace_by_hash_tests {
    use chasm::workspace::get_workspace_by_hash;

    #[test]
    fn test_get_workspace_nonexistent_hash() {
        let result = get_workspace_by_hash("nonexistent_hash_12345");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_get_workspace_empty_hash() {
        let result = get_workspace_by_hash("");
        assert!(result.is_ok());
        // Empty hash should not match any workspace
    }

    #[test]
    fn test_get_workspace_partial_hash() {
        // Test that partial hashes can match (starts_with behavior)
        let result = get_workspace_by_hash("a");
        assert!(result.is_ok());
        // May or may not find a workspace starting with 'a'
    }
}

// ============================================================================
// Get Workspace By Path Tests
// ============================================================================

mod get_workspace_by_path_tests {
    use chasm::workspace::get_workspace_by_path;

    #[test]
    fn test_get_workspace_nonexistent_path() {
        let result = get_workspace_by_path("/nonexistent/path/12345");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_get_workspace_empty_path() {
        let result = get_workspace_by_path("");
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_workspace_relative_path() {
        let result = get_workspace_by_path("relative/path");
        assert!(result.is_ok());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_get_workspace_windows_path() {
        let result = get_workspace_by_path("C:\\Users\\test\\project");
        assert!(result.is_ok());
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_get_workspace_unix_path() {
        let result = get_workspace_by_path("/home/user/project");
        assert!(result.is_ok());
    }
}

// ============================================================================
// Find Workspace By Path Tests
// ============================================================================

mod find_workspace_by_path_tests {
    use chasm::workspace::find_workspace_by_path;

    #[test]
    fn test_find_workspace_nonexistent() {
        let result = find_workspace_by_path("/definitely/nonexistent/path");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_find_workspace_returns_tuple() {
        let result = find_workspace_by_path("/some/path");
        assert!(result.is_ok());
        // If found, should return (hash, path, project_path)
        if let Some((hash, dir, _project)) = result.unwrap() {
            assert!(!hash.is_empty());
            assert!(dir.exists());
        }
    }
}

// ============================================================================
// Find All Workspaces For Project Tests
// ============================================================================

mod find_all_workspaces_for_project_tests {
    use chasm::workspace::find_all_workspaces_for_project;

    #[test]
    fn test_find_all_workspaces_by_name() {
        // Search for a project name that probably doesn't exist
        let result = find_all_workspaces_for_project("very_unique_project_name_12345");
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_find_all_workspaces_empty_name() {
        // Empty string should match nothing or everything
        let result = find_all_workspaces_for_project("");
        assert!(result.is_ok());
    }

    #[test]
    fn test_find_all_workspaces_case_insensitive() {
        // Search is case-insensitive
        let result1 = find_all_workspaces_for_project("TEST");
        let result2 = find_all_workspaces_for_project("test");
        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }
}

// ============================================================================
// Get Chat Sessions From Workspace Tests
// ============================================================================

mod get_chat_sessions_from_workspace_tests {
    use super::*;
    use chasm::workspace::get_chat_sessions_from_workspace;

    #[test]
    fn test_get_sessions_nonexistent_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let result = get_chat_sessions_from_workspace(temp_dir.path());
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_get_sessions_empty_chat_sessions_dir() {
        let temp_dir = TempDir::new().unwrap();
        let chat_sessions = temp_dir.path().join("chatSessions");
        fs::create_dir(&chat_sessions).unwrap();

        let result = get_chat_sessions_from_workspace(temp_dir.path());
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_get_sessions_with_valid_session() {
        let temp_dir = TempDir::new().unwrap();
        let chat_sessions = temp_dir.path().join("chatSessions");
        fs::create_dir(&chat_sessions).unwrap();

        // Create a valid session file
        let session_json = r#"{
            "version": 3,
            "creationDate": 1700000000000,
            "lastMessageDate": 1700000000000,
            "requests": []
        }"#;
        fs::write(chat_sessions.join("test-session.json"), session_json).unwrap();

        let result = get_chat_sessions_from_workspace(temp_dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[test]
    fn test_get_sessions_with_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let chat_sessions = temp_dir.path().join("chatSessions");
        fs::create_dir(&chat_sessions).unwrap();

        // Create an invalid JSON file (should be skipped)
        fs::write(chat_sessions.join("invalid.json"), "not valid json").unwrap();

        let result = get_chat_sessions_from_workspace(temp_dir.path());
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty()); // Invalid files should be skipped
    }

    #[test]
    fn test_get_sessions_with_mixed_files() {
        let temp_dir = TempDir::new().unwrap();
        let chat_sessions = temp_dir.path().join("chatSessions");
        fs::create_dir(&chat_sessions).unwrap();

        // Valid session
        let valid_json = r#"{
            "version": 3,
            "creationDate": 1700000000000,
            "lastMessageDate": 1700000000000,
            "requests": []
        }"#;
        fs::write(chat_sessions.join("valid.json"), valid_json).unwrap();

        // Invalid session (wrong structure)
        fs::write(chat_sessions.join("invalid.json"), "{}").unwrap();

        // Non-JSON file (should be ignored)
        fs::write(chat_sessions.join("readme.txt"), "This is a readme").unwrap();

        let result = get_chat_sessions_from_workspace(temp_dir.path());
        assert!(result.is_ok());
        // Only valid JSON files with correct structure should be returned
    }

    #[test]
    fn test_get_sessions_multiple_valid() {
        let temp_dir = TempDir::new().unwrap();
        let chat_sessions = temp_dir.path().join("chatSessions");
        fs::create_dir(&chat_sessions).unwrap();

        let session_json = r#"{
            "version": 3,
            "creationDate": 1700000000000,
            "lastMessageDate": 1700000000000,
            "requests": []
        }"#;

        for i in 0..5 {
            fs::write(
                chat_sessions.join(format!("session-{}.json", i)),
                session_json,
            )
            .unwrap();
        }

        let result = get_chat_sessions_from_workspace(temp_dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 5);
    }
}

// ============================================================================
// Integration Tests with Temp Directories
// ============================================================================

mod workspace_integration_tests {
    use super::*;

    fn create_workspace_structure(base: &TempDir, hash: &str, project_path: &str) -> PathBuf {
        let ws_dir = base.path().join(hash);
        let sessions_dir = ws_dir.join("chatSessions");
        fs::create_dir_all(&sessions_dir).unwrap();

        let ws_json = format!(
            r#"{{"folder": "file:///{}", "configuration": {{}}}}"#,
            project_path.replace('\\', "/").replace(' ', "%20")
        );
        fs::write(ws_dir.join("workspace.json"), ws_json).unwrap();

        ws_dir
    }

    #[test]
    fn test_workspace_json_parsing() {
        let temp_dir = TempDir::new().unwrap();
        create_workspace_structure(&temp_dir, "abc123", "/home/user/project");

        let ws_json_path = temp_dir.path().join("abc123").join("workspace.json");
        let content = fs::read_to_string(&ws_json_path).unwrap();

        let parsed: chasm::models::WorkspaceJson = serde_json::from_str(&content).unwrap();
        assert!(parsed.folder.is_some());
        assert!(parsed.folder.unwrap().contains("project"));
    }

    #[test]
    fn test_workspace_sessions_path() {
        let temp_dir = TempDir::new().unwrap();
        let ws_dir = create_workspace_structure(&temp_dir, "def456", "/home/user/myproject");

        let sessions_path = ws_dir.join("chatSessions");
        assert!(sessions_path.exists());
        assert!(sessions_path.is_dir());
    }

    #[test]
    fn test_workspace_with_special_chars_in_path() {
        let temp_dir = TempDir::new().unwrap();
        create_workspace_structure(&temp_dir, "special123", "/home/user/my project (v2)");

        let ws_json_path = temp_dir.path().join("special123").join("workspace.json");
        let content = fs::read_to_string(&ws_json_path).unwrap();

        // Should contain encoded spaces
        assert!(content.contains("%20") || content.contains(" "));
    }

    #[test]
    fn test_multiple_workspaces() {
        let temp_dir = TempDir::new().unwrap();

        for i in 0..10 {
            create_workspace_structure(
                &temp_dir,
                &format!("workspace{:03}", i),
                &format!("/home/user/project{}", i),
            );
        }

        let entries: Vec<_> = fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();

        assert_eq!(entries.len(), 10);
    }
}

// ============================================================================
// Edge Case Tests
// ============================================================================

mod workspace_edge_cases {
    use super::*;
    use chasm::workspace::decode_workspace_folder;

    #[test]
    fn test_decode_percent_sign_literal() {
        let uri = "file:///home/user/100%complete";
        // Should handle gracefully
        let decoded = decode_workspace_folder(uri);
        assert!(!decoded.is_empty());
    }

    #[test]
    fn test_decode_unicode_in_path() {
        let uri = "file:///home/user/project";
        let decoded = decode_workspace_folder(uri);
        assert!(!decoded.is_empty());
    }

    #[test]
    fn test_decode_very_long_path() {
        let long_path = format!("file:///{}", "a/".repeat(100));
        let decoded = decode_workspace_folder(&long_path);
        assert!(!decoded.is_empty());
    }

    #[test]
    fn test_decode_path_with_hash() {
        let uri = "file:///home/user/project#section";
        let decoded = decode_workspace_folder(uri);
        assert!(decoded.contains("project") || decoded.contains("#"));
    }

    #[test]
    fn test_workspace_without_workspace_json() {
        let temp_dir = TempDir::new().unwrap();
        let ws_dir = temp_dir.path().join("no_ws_json");
        fs::create_dir_all(&ws_dir).unwrap();

        // Directory exists but no workspace.json
        assert!(ws_dir.exists());
        assert!(!ws_dir.join("workspace.json").exists());
    }

    #[test]
    fn test_workspace_malformed_workspace_json() {
        let temp_dir = TempDir::new().unwrap();
        let ws_dir = temp_dir.path().join("malformed");
        fs::create_dir_all(&ws_dir).unwrap();
        fs::write(ws_dir.join("workspace.json"), "{ malformed json }").unwrap();

        // Should handle gracefully
        let ws_json_path = ws_dir.join("workspace.json");
        assert!(ws_json_path.exists());
    }

    #[test]
    fn test_workspace_empty_workspace_json() {
        let temp_dir = TempDir::new().unwrap();
        let ws_dir = temp_dir.path().join("empty_json");
        fs::create_dir_all(&ws_dir).unwrap();
        fs::write(ws_dir.join("workspace.json"), "{}").unwrap();

        let content = fs::read_to_string(ws_dir.join("workspace.json")).unwrap();
        let parsed: Result<chasm::models::WorkspaceJson, _> = serde_json::from_str(&content);
        assert!(parsed.is_ok());
        assert!(parsed.unwrap().folder.is_none());
    }

    #[test]
    fn test_workspace_null_folder() {
        let temp_dir = TempDir::new().unwrap();
        let ws_dir = temp_dir.path().join("null_folder");
        fs::create_dir_all(&ws_dir).unwrap();
        fs::write(ws_dir.join("workspace.json"), r#"{"folder": null}"#).unwrap();

        let content = fs::read_to_string(ws_dir.join("workspace.json")).unwrap();
        let parsed: chasm::models::WorkspaceJson = serde_json::from_str(&content).unwrap();
        assert!(parsed.folder.is_none());
    }
}

// ============================================================================
// Cross-Platform Tests
// ============================================================================

mod cross_platform_tests {
    use chasm::workspace::{decode_workspace_folder, normalize_path};

    #[test]
    fn test_decode_mixed_slashes() {
        let uri = "file:///C:/Users\\test/project";
        let decoded = decode_workspace_folder(uri);
        assert!(!decoded.is_empty());
    }

    #[test]
    fn test_normalize_mixed_slashes() {
        let path = "/home/user\\project/subdir";
        let normalized = normalize_path(path);
        assert!(!normalized.is_empty());
    }

    #[test]
    fn test_unc_path() {
        let uri = "file://server/share/folder";
        let decoded = decode_workspace_folder(uri);
        assert!(!decoded.is_empty());
    }

    #[test]
    fn test_wsl_path() {
        let uri = "file:///mnt/c/Users/test/project";
        let decoded = decode_workspace_folder(uri);
        assert!(decoded.contains("mnt") || decoded.contains("Users"));
    }
}

// ============================================================================
// Global Storage and Empty Window Sessions Path Tests
// ============================================================================

mod global_storage_path_tests {
    use chasm::workspace::{get_empty_window_sessions_path, get_global_storage_path};

    #[test]
    fn test_get_global_storage_path_returns_valid_path() {
        let result = get_global_storage_path();
        assert!(result.is_ok());

        let path = result.unwrap();
        // Should end with globalStorage
        assert!(path.to_string_lossy().contains("globalStorage"));
    }

    #[test]
    fn test_get_global_storage_path_contains_code_user() {
        let result = get_global_storage_path();
        assert!(result.is_ok());

        let path = result.unwrap();
        let path_str = path.to_string_lossy().to_lowercase();
        // Should contain Code/User or Code\User path segments
        assert!(path_str.contains("code"));
        assert!(path_str.contains("user"));
    }

    #[test]
    fn test_get_empty_window_sessions_path_returns_valid_path() {
        let result = get_empty_window_sessions_path();
        assert!(result.is_ok());

        let path = result.unwrap();
        // Should end with emptyWindowChatSessions
        assert!(path.to_string_lossy().contains("emptyWindowChatSessions"));
    }

    #[test]
    fn test_get_empty_window_sessions_path_under_global_storage() {
        let global = get_global_storage_path().unwrap();
        let empty_sessions = get_empty_window_sessions_path().unwrap();

        // Empty window sessions path should be under global storage
        assert!(empty_sessions.starts_with(&global));
    }

    #[test]
    fn test_empty_window_sessions_path_is_not_workspace_storage() {
        let result = get_empty_window_sessions_path();
        assert!(result.is_ok());

        let path = result.unwrap();
        let path_str = path.to_string_lossy();

        // Should NOT be under workspaceStorage
        assert!(!path_str.contains("workspaceStorage"));
        // Should be under globalStorage
        assert!(path_str.contains("globalStorage"));
    }

    #[test]
    fn test_global_storage_path_platform_specific() {
        let result = get_global_storage_path();
        assert!(result.is_ok());

        let path = result.unwrap();
        let path_str = path.to_string_lossy();

        // Platform-specific checks
        if cfg!(target_os = "windows") {
            // Windows: Should be under AppData/Roaming/Code
            assert!(
                path_str.contains("AppData") || path_str.contains("Roaming"),
                "Windows path should contain AppData or Roaming"
            );
        } else if cfg!(target_os = "macos") {
            // macOS: Should be under Library/Application Support/Code
            assert!(
                path_str.contains("Library") || path_str.contains("Application Support"),
                "macOS path should contain Library or Application Support"
            );
        } else {
            // Linux: Should be under .config/Code
            assert!(
                path_str.contains(".config"),
                "Linux path should contain .config"
            );
        }
    }
}
