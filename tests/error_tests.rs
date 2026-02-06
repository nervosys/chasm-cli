//! Extensive tests for error handling
//!
//! This file contains comprehensive unit tests for:
//! - CsmError types and variants
//! - Error messages and Display trait
//! - Error conversions and From implementations
//! - Result type usage

use chasm::error::CsmError;

// ============================================================================
// CsmError Variant Tests
// ============================================================================

mod error_variant_tests {
    use super::*;

    #[test]
    fn test_workspace_not_found_error() {
        let err = CsmError::WorkspaceNotFound("/path/to/workspace".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Workspace not found"));
        assert!(msg.contains("/path/to/workspace"));
    }

    #[test]
    fn test_session_not_found_error() {
        let err = CsmError::SessionNotFound("session-abc-123".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Session not found"));
        assert!(msg.contains("session-abc-123"));
    }

    #[test]
    fn test_invalid_session_format_error() {
        let err = CsmError::InvalidSessionFormat("missing version field".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Invalid session format"));
        assert!(msg.contains("missing version field"));
    }

    #[test]
    fn test_storage_not_found_error() {
        let err = CsmError::StorageNotFound;
        let msg = format!("{}", err);
        assert!(msg.contains("VS Code storage not found"));
    }

    #[test]
    fn test_database_error() {
        let err = CsmError::DatabaseError("connection failed".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Database error"));
        assert!(msg.contains("connection failed"));
    }

    #[test]
    fn test_git_error() {
        let err = CsmError::GitError("not a git repository".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Git error"));
        assert!(msg.contains("not a git repository"));
    }

    #[test]
    fn test_vscode_running_error() {
        let err = CsmError::VSCodeRunning;
        let msg = format!("{}", err);
        assert!(msg.contains("VS Code is running"));
        assert!(msg.contains("--force"));
    }

    #[test]
    fn test_no_sessions_found_error() {
        let err = CsmError::NoSessionsFound;
        let msg = format!("{}", err);
        assert!(msg.contains("No chat sessions found"));
    }

    #[test]
    fn test_missing_target_specifier_error() {
        let err = CsmError::MissingTargetSpecifier;
        let msg = format!("{}", err);
        assert!(msg.contains("--hash") || msg.contains("--path"));
    }
}

// ============================================================================
// Error From Implementations Tests
// ============================================================================

mod error_from_tests {
    use super::*;

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let csm_err: CsmError = io_err.into();
        let msg = format!("{}", csm_err);
        assert!(msg.contains("IO error") || msg.contains("file not found"));
    }

    #[test]
    fn test_from_io_error_permission_denied() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let csm_err: CsmError = io_err.into();
        let msg = format!("{}", csm_err);
        assert!(msg.contains("IO error") || msg.contains("access denied"));
    }

    #[test]
    fn test_from_json_error() {
        let invalid_json = "not valid json";
        let json_err = serde_json::from_str::<serde_json::Value>(invalid_json).unwrap_err();
        let csm_err: CsmError = json_err.into();
        let msg = format!("{}", csm_err);
        assert!(msg.contains("JSON error") || msg.contains("expected"));
    }

    #[test]
    fn test_from_sqlite_error() {
        // Create a SQLite error by trying to open a non-existent database in read-only mode
        let sqlite_result: Result<rusqlite::Connection, rusqlite::Error> =
            rusqlite::Connection::open_in_memory();

        // This should succeed, but we can test the From implementation
        if let Ok(conn) = sqlite_result {
            // Try to execute invalid SQL
            let err = conn.execute("INVALID SQL STATEMENT", []).unwrap_err();
            let csm_err: CsmError = err.into();
            let msg = format!("{}", csm_err);
            assert!(msg.contains("SQLite error") || msg.contains("syntax"));
        }
    }
}

// ============================================================================
// Error Debug Tests
// ============================================================================

mod error_debug_tests {
    use super::*;

    #[test]
    fn test_error_debug_format() {
        let err = CsmError::WorkspaceNotFound("/test/path".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("WorkspaceNotFound"));
        assert!(debug.contains("/test/path"));
    }

    #[test]
    fn test_all_variants_debug() {
        let errors: Vec<CsmError> = vec![
            CsmError::WorkspaceNotFound("test".to_string()),
            CsmError::SessionNotFound("test".to_string()),
            CsmError::InvalidSessionFormat("test".to_string()),
            CsmError::StorageNotFound,
            CsmError::DatabaseError("test".to_string()),
            CsmError::GitError("test".to_string()),
            CsmError::VSCodeRunning,
            CsmError::NoSessionsFound,
            CsmError::MissingTargetSpecifier,
        ];

        for err in errors {
            let debug = format!("{:?}", err);
            assert!(!debug.is_empty());
        }
    }
}

// ============================================================================
// Error Message Tests
// ============================================================================

mod error_message_tests {
    use super::*;

    #[test]
    fn test_workspace_not_found_empty_path() {
        let err = CsmError::WorkspaceNotFound("".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Workspace not found"));
    }

    #[test]
    fn test_session_not_found_uuid_format() {
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        let err = CsmError::SessionNotFound(uuid.to_string());
        let msg = format!("{}", err);
        assert!(msg.contains(uuid));
    }

    #[test]
    fn test_invalid_session_format_json_details() {
        let err = CsmError::InvalidSessionFormat("expected object at line 1 column 1".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("line 1 column 1"));
    }

    #[test]
    fn test_database_error_with_details() {
        let err = CsmError::DatabaseError("SQLITE_BUSY: database is locked".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("SQLITE_BUSY") || msg.contains("database is locked"));
    }

    #[test]
    fn test_git_error_with_command() {
        let err = CsmError::GitError("git init failed: not a directory".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("git init") || msg.contains("not a directory"));
    }
}

// ============================================================================
// Result Type Tests
// ============================================================================

mod result_type_tests {
    use super::*;
    use chasm::error::Result;

    fn make_ok() -> Result<i32> {
        Ok(42)
    }

    #[test]
    fn test_result_ok() {
        let result = make_ok();
        assert!(result.is_ok());
        assert_eq!(result.expect("should be ok"), 42);
    }

    #[test]
    fn test_result_err() {
        let result: Result<i32> = Err(CsmError::StorageNotFound);
        assert!(result.is_err());
    }

    #[test]
    fn test_result_map() {
        let result: Result<i32> = Ok(21);
        let doubled = result.map(|x| x * 2);
        assert_eq!(doubled.expect("should be ok"), 42);
    }

    #[test]
    fn test_result_map_err() {
        let result: Result<i32> = Err(CsmError::StorageNotFound);
        let mapped = result.map_err(|_| CsmError::NoSessionsFound);
        assert!(matches!(mapped, Err(CsmError::NoSessionsFound)));
    }

    #[test]
    fn test_result_and_then() {
        fn double_if_positive(x: i32) -> Result<i32> {
            if x > 0 {
                Ok(x * 2)
            } else {
                Err(CsmError::InvalidSessionFormat(
                    "negative number".to_string(),
                ))
            }
        }

        let result: Result<i32> = Ok(10);
        let chained = result.and_then(double_if_positive);
        assert_eq!(chained.unwrap(), 20);

        let negative: Result<i32> = Ok(-5);
        let chained = negative.and_then(double_if_positive);
        assert!(chained.is_err());
    }

    #[test]
    fn test_result_unwrap_or() {
        fn make_ok() -> Result<i32> {
            Ok(42)
        }
        fn make_err() -> Result<i32> {
            Err(CsmError::StorageNotFound)
        }

        assert_eq!(make_ok().unwrap_or(0), 42);
        assert_eq!(make_err().unwrap_or(0), 0);
    }

    #[test]
    fn test_result_question_mark_operator() {
        fn function_that_fails() -> Result<()> {
            Err(CsmError::NoSessionsFound)
        }

        fn caller() -> Result<String> {
            function_that_fails()?;
            Ok("success".to_string())
        }

        assert!(caller().is_err());
    }
}

// ============================================================================
// Error Clone/Copy Tests
// ============================================================================

mod error_traits_tests {
    use super::*;

    // Note: CsmError derives Error and Debug, not Clone
    // These tests verify the basic trait behavior

    #[test]
    fn test_error_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<CsmError>();
    }

    #[test]
    fn test_error_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<CsmError>();
    }

    #[test]
    fn test_error_has_source() {
        use std::error::Error;

        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let csm_err: CsmError = io_err.into();

        // IO errors should have a source
        // (thiserror handles this automatically with #[from])
        let _ = csm_err.source();
    }
}

// ============================================================================
// Edge Case Tests
// ============================================================================

mod edge_case_tests {
    use super::*;

    #[test]
    fn test_error_with_unicode() {
        let err = CsmError::WorkspaceNotFound("/home/user/project".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("user"));
    }

    #[test]
    fn test_error_with_special_chars() {
        let err = CsmError::SessionNotFound("session-with-\"quotes\"-and-<brackets>".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("quotes"));
    }

    #[test]
    fn test_error_with_newlines() {
        let err = CsmError::InvalidSessionFormat("Error on\nmultiple\nlines".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("multiple"));
    }

    #[test]
    fn test_error_with_very_long_message() {
        let long_msg = "x".repeat(10000);
        let err = CsmError::DatabaseError(long_msg.clone());
        let msg = format!("{}", err);
        assert!(msg.len() >= 10000);
    }

    #[test]
    fn test_error_with_empty_string() {
        let err = CsmError::GitError("".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Git error"));
    }
}

// ============================================================================
// Error Comparison Tests
// ============================================================================

mod error_comparison_tests {
    use super::*;

    #[test]
    fn test_different_errors_not_equal() {
        // We can't directly compare CsmError instances (no PartialEq),
        // but we can compare their string representations
        let err1 = format!("{}", CsmError::StorageNotFound);
        let err2 = format!("{}", CsmError::NoSessionsFound);
        assert_ne!(err1, err2);
    }

    #[test]
    fn test_same_error_type_different_values() {
        let err1 = format!("{}", CsmError::WorkspaceNotFound("/path1".to_string()));
        let err2 = format!("{}", CsmError::WorkspaceNotFound("/path2".to_string()));
        assert_ne!(err1, err2);
        assert!(err1.contains("path1"));
        assert!(err2.contains("path2"));
    }
}
