// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Synchronization Module
//!
//! Provides bidirectional synchronization between CSM and provider-native storage.

pub mod bidirectional;

pub use bidirectional::{
    compute_session_hash, BidirectionalSyncConfig, BidirectionalSyncEngine, ChangeOrigin,
    ChangeType, ConflictStrategy, ConflictType, ConflictVersion, EntityType, ProviderSyncAdapter,
    SessionSyncState, SyncChange, SyncConflict, SyncResult, SyncStatus, VSCodeSyncAdapter,
};
