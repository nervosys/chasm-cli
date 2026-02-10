// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Application state for the API server

use std::path::PathBuf;
use std::sync::Mutex;

use crate::database::ChatDatabase;

/// Shared application state
pub struct AppState {
    pub db: Mutex<ChatDatabase>,
    #[allow(dead_code)] // Reserved for future use (e.g., reopening database)
    pub db_path: PathBuf,
}

impl AppState {
    pub fn new(db: ChatDatabase, db_path: PathBuf) -> Self {
        Self {
            db: Mutex::new(db),
            db_path,
        }
    }
}
