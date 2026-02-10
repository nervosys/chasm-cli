// Copyright (c) 2024-2027 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Role-Based Access Control (RBAC) module
//!
//! Provides roles, permissions, and access control for team workspaces.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

// ============================================================================
// Roles and Permissions
// ============================================================================

/// Team role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Team owner - full access
    Owner,
    /// Administrator - can manage team and members
    Admin,
    /// Regular member - can view and contribute
    Member,
    /// Viewer - read-only access
    Viewer,
    /// Guest - limited access to specific resources
    Guest,
}

impl Role {
    /// Get default permissions for this role
    pub fn default_permissions(&self) -> HashSet<Permission> {
        match self {
            Role::Owner => Permission::all(),
            Role::Admin => {
                let mut perms = Permission::all();
                perms.remove(&Permission::DeleteTeam);
                perms.remove(&Permission::TransferOwnership);
                perms
            }
            Role::Member => {
                let mut perms = HashSet::new();
                perms.insert(Permission::ViewTeam);
                perms.insert(Permission::ViewMembers);
                perms.insert(Permission::ViewSessions);
                perms.insert(Permission::CreateSession);
                perms.insert(Permission::EditOwnSessions);
                perms.insert(Permission::DeleteOwnSessions);
                perms.insert(Permission::ShareSessions);
                perms.insert(Permission::AddComments);
                perms.insert(Permission::ViewAnalytics);
                perms.insert(Permission::ViewActivityFeed);
                perms
            }
            Role::Viewer => {
                let mut perms = HashSet::new();
                perms.insert(Permission::ViewTeam);
                perms.insert(Permission::ViewMembers);
                perms.insert(Permission::ViewSessions);
                perms.insert(Permission::ViewAnalytics);
                perms.insert(Permission::ViewActivityFeed);
                perms
            }
            Role::Guest => {
                let mut perms = HashSet::new();
                perms.insert(Permission::ViewSessions);
                perms
            }
        }
    }

    /// Check if this role has a permission
    pub fn has_permission(&self, permission: Permission) -> bool {
        self.default_permissions().contains(&permission)
    }
}

/// Granular permission
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    // Team management
    ViewTeam,
    EditTeam,
    DeleteTeam,
    TransferOwnership,

    // Member management
    ViewMembers,
    InviteMembers,
    RemoveMembers,
    EditMemberRoles,

    // Session management
    ViewSessions,
    CreateSession,
    EditOwnSessions,
    EditAllSessions,
    DeleteOwnSessions,
    DeleteAllSessions,
    ShareSessions,
    ExportSessions,

    // Collaboration
    AddComments,
    EditOwnComments,
    EditAllComments,
    DeleteOwnComments,
    DeleteAllComments,

    // Analytics
    ViewAnalytics,
    ExportAnalytics,
    ConfigureAnalytics,

    // Activity
    ViewActivityFeed,

    // Settings
    EditTeamSettings,
    ManageIntegrations,
    ManageWebhooks,

    // Admin
    ViewAuditLog,
    ManageRetentionPolicy,
}

impl Permission {
    /// Get all permissions
    pub fn all() -> HashSet<Permission> {
        use Permission::*;
        [
            ViewTeam,
            EditTeam,
            DeleteTeam,
            TransferOwnership,
            ViewMembers,
            InviteMembers,
            RemoveMembers,
            EditMemberRoles,
            ViewSessions,
            CreateSession,
            EditOwnSessions,
            EditAllSessions,
            DeleteOwnSessions,
            DeleteAllSessions,
            ShareSessions,
            ExportSessions,
            AddComments,
            EditOwnComments,
            EditAllComments,
            DeleteOwnComments,
            DeleteAllComments,
            ViewAnalytics,
            ExportAnalytics,
            ConfigureAnalytics,
            ViewActivityFeed,
            EditTeamSettings,
            ManageIntegrations,
            ManageWebhooks,
            ViewAuditLog,
            ManageRetentionPolicy,
        ]
        .into_iter()
        .collect()
    }

    /// Get permission description
    pub fn description(&self) -> &'static str {
        use Permission::*;
        match self {
            ViewTeam => "View team information",
            EditTeam => "Edit team name and description",
            DeleteTeam => "Delete the team",
            TransferOwnership => "Transfer team ownership",
            ViewMembers => "View team members",
            InviteMembers => "Invite new members",
            RemoveMembers => "Remove members from team",
            EditMemberRoles => "Change member roles",
            ViewSessions => "View sessions",
            CreateSession => "Create new sessions",
            EditOwnSessions => "Edit own sessions",
            EditAllSessions => "Edit any session",
            DeleteOwnSessions => "Delete own sessions",
            DeleteAllSessions => "Delete any session",
            ShareSessions => "Share sessions with team",
            ExportSessions => "Export sessions",
            AddComments => "Add comments",
            EditOwnComments => "Edit own comments",
            EditAllComments => "Edit any comment",
            DeleteOwnComments => "Delete own comments",
            DeleteAllComments => "Delete any comment",
            ViewAnalytics => "View analytics",
            ExportAnalytics => "Export analytics data",
            ConfigureAnalytics => "Configure analytics settings",
            ViewActivityFeed => "View activity feed",
            EditTeamSettings => "Edit team settings",
            ManageIntegrations => "Manage integrations",
            ManageWebhooks => "Manage webhooks",
            ViewAuditLog => "View audit log",
            ManageRetentionPolicy => "Manage data retention",
        }
    }
}

/// Role assignment for a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleAssignment {
    /// User ID
    pub user_id: Uuid,
    /// Team ID
    pub team_id: Uuid,
    /// Assigned role
    pub role: Role,
    /// Custom permission overrides (additions)
    pub granted_permissions: HashSet<Permission>,
    /// Custom permission overrides (removals)
    pub revoked_permissions: HashSet<Permission>,
}

impl RoleAssignment {
    /// Create a new role assignment
    pub fn new(user_id: Uuid, team_id: Uuid, role: Role) -> Self {
        Self {
            user_id,
            team_id,
            role,
            granted_permissions: HashSet::new(),
            revoked_permissions: HashSet::new(),
        }
    }

    /// Get effective permissions
    pub fn effective_permissions(&self) -> HashSet<Permission> {
        let mut perms = self.role.default_permissions();

        // Add granted permissions
        for perm in &self.granted_permissions {
            perms.insert(*perm);
        }

        // Remove revoked permissions
        for perm in &self.revoked_permissions {
            perms.remove(perm);
        }

        perms
    }

    /// Check if user has a specific permission
    pub fn has_permission(&self, permission: Permission) -> bool {
        self.effective_permissions().contains(&permission)
    }

    /// Grant an additional permission
    pub fn grant(&mut self, permission: Permission) {
        self.revoked_permissions.remove(&permission);
        self.granted_permissions.insert(permission);
    }

    /// Revoke a permission
    pub fn revoke(&mut self, permission: Permission) {
        self.granted_permissions.remove(&permission);
        self.revoked_permissions.insert(permission);
    }
}

// ============================================================================
// Access Control
// ============================================================================

/// Resource being accessed
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Resource {
    Team { team_id: Uuid },
    Member { team_id: Uuid, member_id: Uuid },
    Session { team_id: Uuid, session_id: String, owner_id: Uuid },
    Comment { team_id: Uuid, comment_id: String, author_id: Uuid },
    Analytics { team_id: Uuid },
    Settings { team_id: Uuid },
}

/// Action being performed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    View,
    Create,
    Edit,
    Delete,
    Share,
    Export,
    Manage,
}

/// Access decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessDecision {
    Allow,
    Deny,
}

/// Access control manager
pub struct AccessControl {
    /// Role assignments by (team_id, user_id)
    assignments: HashMap<(Uuid, Uuid), RoleAssignment>,
}

impl AccessControl {
    /// Create a new access control manager
    pub fn new() -> Self {
        Self {
            assignments: HashMap::new(),
        }
    }

    /// Add a role assignment
    pub fn assign_role(&mut self, assignment: RoleAssignment) {
        self.assignments.insert(
            (assignment.team_id, assignment.user_id),
            assignment,
        );
    }

    /// Remove a role assignment
    pub fn remove_assignment(&mut self, team_id: Uuid, user_id: Uuid) {
        self.assignments.remove(&(team_id, user_id));
    }

    /// Get role assignment
    pub fn get_assignment(&self, team_id: Uuid, user_id: Uuid) -> Option<&RoleAssignment> {
        self.assignments.get(&(team_id, user_id))
    }

    /// Check if a user can perform an action on a resource
    pub fn check(&self, user_id: Uuid, resource: &Resource, action: Action) -> AccessDecision {
        let team_id = match resource {
            Resource::Team { team_id } => *team_id,
            Resource::Member { team_id, .. } => *team_id,
            Resource::Session { team_id, .. } => *team_id,
            Resource::Comment { team_id, .. } => *team_id,
            Resource::Analytics { team_id } => *team_id,
            Resource::Settings { team_id } => *team_id,
        };

        // Get user's role assignment
        let assignment = match self.get_assignment(team_id, user_id) {
            Some(a) => a,
            None => return AccessDecision::Deny,
        };

        // Map resource and action to required permission
        let required_permission = self.get_required_permission(user_id, resource, action);

        if assignment.has_permission(required_permission) {
            AccessDecision::Allow
        } else {
            AccessDecision::Deny
        }
    }

    /// Get the required permission for a resource/action combination
    fn get_required_permission(
        &self,
        user_id: Uuid,
        resource: &Resource,
        action: Action,
    ) -> Permission {
        match (resource, action) {
            // Team
            (Resource::Team { .. }, Action::View) => Permission::ViewTeam,
            (Resource::Team { .. }, Action::Edit) => Permission::EditTeam,
            (Resource::Team { .. }, Action::Delete) => Permission::DeleteTeam,

            // Members
            (Resource::Member { .. }, Action::View) => Permission::ViewMembers,
            (Resource::Member { .. }, Action::Create) => Permission::InviteMembers,
            (Resource::Member { .. }, Action::Delete) => Permission::RemoveMembers,
            (Resource::Member { .. }, Action::Edit) => Permission::EditMemberRoles,

            // Sessions
            (Resource::Session { owner_id: _, .. }, Action::View) => Permission::ViewSessions,
            (Resource::Session { .. }, Action::Create) => Permission::CreateSession,
            (Resource::Session { owner_id, .. }, Action::Edit) => {
                if *owner_id == user_id {
                    Permission::EditOwnSessions
                } else {
                    Permission::EditAllSessions
                }
            }
            (Resource::Session { owner_id, .. }, Action::Delete) => {
                if *owner_id == user_id {
                    Permission::DeleteOwnSessions
                } else {
                    Permission::DeleteAllSessions
                }
            }
            (Resource::Session { .. }, Action::Share) => Permission::ShareSessions,
            (Resource::Session { .. }, Action::Export) => Permission::ExportSessions,

            // Comments
            (Resource::Comment { author_id: _, .. }, Action::Create) => Permission::AddComments,
            (Resource::Comment { author_id, .. }, Action::Edit) => {
                if *author_id == user_id {
                    Permission::EditOwnComments
                } else {
                    Permission::EditAllComments
                }
            }
            (Resource::Comment { author_id, .. }, Action::Delete) => {
                if *author_id == user_id {
                    Permission::DeleteOwnComments
                } else {
                    Permission::DeleteAllComments
                }
            }

            // Analytics
            (Resource::Analytics { .. }, Action::View) => Permission::ViewAnalytics,
            (Resource::Analytics { .. }, Action::Export) => Permission::ExportAnalytics,
            (Resource::Analytics { .. }, Action::Manage) => Permission::ConfigureAnalytics,

            // Settings
            (Resource::Settings { .. }, Action::View) => Permission::ViewTeam,
            (Resource::Settings { .. }, Action::Edit) => Permission::EditTeamSettings,

            // Default deny
            _ => Permission::ViewTeam, // Will be denied if user doesn't have it
        }
    }
}

impl Default for AccessControl {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_permissions() {
        assert!(Role::Owner.has_permission(Permission::DeleteTeam));
        assert!(Role::Admin.has_permission(Permission::InviteMembers));
        assert!(!Role::Admin.has_permission(Permission::DeleteTeam));
        assert!(Role::Member.has_permission(Permission::ViewSessions));
        assert!(!Role::Member.has_permission(Permission::EditAllSessions));
        assert!(Role::Viewer.has_permission(Permission::ViewSessions));
        assert!(!Role::Viewer.has_permission(Permission::CreateSession));
    }

    #[test]
    fn test_role_assignment() {
        let user_id = Uuid::new_v4();
        let team_id = Uuid::new_v4();
        let mut assignment = RoleAssignment::new(user_id, team_id, Role::Member);

        assert!(assignment.has_permission(Permission::CreateSession));
        assert!(!assignment.has_permission(Permission::EditAllSessions));

        // Grant additional permission
        assignment.grant(Permission::EditAllSessions);
        assert!(assignment.has_permission(Permission::EditAllSessions));

        // Revoke a default permission
        assignment.revoke(Permission::CreateSession);
        assert!(!assignment.has_permission(Permission::CreateSession));
    }

    #[test]
    fn test_access_control() {
        let mut ac = AccessControl::new();
        let user_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let team_id = Uuid::new_v4();

        ac.assign_role(RoleAssignment::new(user_id, team_id, Role::Member));

        // Can view sessions
        let resource = Resource::Session {
            team_id,
            session_id: "session-1".to_string(),
            owner_id,
        };
        assert_eq!(ac.check(user_id, &resource, Action::View), AccessDecision::Allow);

        // Cannot edit others' sessions
        assert_eq!(ac.check(user_id, &resource, Action::Edit), AccessDecision::Deny);

        // Can edit own sessions
        let own_resource = Resource::Session {
            team_id,
            session_id: "session-2".to_string(),
            owner_id: user_id,
        };
        assert_eq!(ac.check(user_id, &own_resource, Action::Edit), AccessDecision::Allow);
    }
}
