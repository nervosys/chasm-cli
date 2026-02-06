// Copyright (c) 2024-2027 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Team management module
//!
//! Provides team workspaces, collaboration, RBAC, and activity tracking.

pub mod workspace;
pub mod rbac;
pub mod activity;
pub mod search;

pub use workspace::*;
pub use rbac::*;
pub use activity::*;
pub use search::*;
