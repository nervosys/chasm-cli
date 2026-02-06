// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Synchronization Module
//!
//! Provides bidirectional synchronization between CSM and provider-native storage.

pub mod bidirectional;

pub use bidirectional::{
    compute_session_hash, BidirectionalSyncConfig, BidirectionalSyncEngine, ChangeOrigin,
    ChangeType, ConflictStrategy, ConflictType, ConflictVersion, EntityType, ProviderSyncAdapter,
    SessionSyncState, SyncChange, SyncConflict, SyncResult, SyncStatus, VSCodeSyncAdapter,
};
