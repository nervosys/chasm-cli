// Copyright (c) 2024-2027 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Team workspace management
//!
//! Provides shared team workspaces with real-time collaboration features.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use super::rbac::{Permission, Role, RoleAssignment};

// ============================================================================
// Types
// ============================================================================

/// Unique identifier for a team
pub type TeamId = Uuid;

/// Unique identifier for a team member
pub type MemberId = Uuid;

/// Team workspace representing a collaborative environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamWorkspace {
    /// Unique team identifier
    pub id: TeamId,
    /// Team name
    pub name: String,
    /// Team description
    pub description: Option<String>,
    /// Team avatar URL
    pub avatar_url: Option<String>,
    /// Team owner ID
    pub owner_id: MemberId,
    /// Team settings
    pub settings: TeamSettings,
    /// Team members with their roles
    pub members: Vec<TeamMember>,
    /// Shared session IDs
    pub shared_sessions: Vec<String>,
    /// Team creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
}

/// Team member information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    /// Member user ID
    pub user_id: MemberId,
    /// Member display name
    pub display_name: String,
    /// Member email
    pub email: String,
    /// Member avatar URL
    pub avatar_url: Option<String>,
    /// Member role in the team
    pub role: Role,
    /// Custom permissions (overrides role defaults)
    pub custom_permissions: Option<Vec<Permission>>,
    /// Join timestamp
    pub joined_at: DateTime<Utc>,
    /// Last active timestamp
    pub last_active: Option<DateTime<Utc>>,
    /// Member status
    pub status: MemberStatus,
}

/// Member status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberStatus {
    /// Active member
    Active,
    /// Invited, pending acceptance
    Invited,
    /// Deactivated by admin
    Deactivated,
    /// Left the team
    Left,
}

/// Team settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamSettings {
    /// Whether new members can view all team sessions
    pub default_session_visibility: SessionVisibility,
    /// Whether to allow session sharing outside team
    pub allow_external_sharing: bool,
    /// Default role for new members
    pub default_member_role: Role,
    /// Require approval for join requests
    pub require_join_approval: bool,
    /// Enable real-time collaboration features
    pub enable_realtime_collaboration: bool,
    /// Session retention policy (days, 0 = forever)
    pub session_retention_days: u32,
    /// Maximum team size (0 = unlimited)
    pub max_members: u32,
}

impl Default for TeamSettings {
    fn default() -> Self {
        Self {
            default_session_visibility: SessionVisibility::TeamOnly,
            allow_external_sharing: false,
            default_member_role: Role::Member,
            require_join_approval: true,
            enable_realtime_collaboration: true,
            session_retention_days: 0,
            max_members: 0,
        }
    }
}

/// Session visibility within team
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionVisibility {
    /// Only the owner can see
    Private,
    /// All team members can see
    TeamOnly,
    /// Anyone with link can see
    Public,
}

/// Team invitation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamInvitation {
    /// Invitation ID
    pub id: Uuid,
    /// Team ID
    pub team_id: TeamId,
    /// Inviter user ID
    pub inviter_id: MemberId,
    /// Invitee email
    pub invitee_email: String,
    /// Assigned role
    pub role: Role,
    /// Invitation message
    pub message: Option<String>,
    /// Expiration timestamp
    pub expires_at: DateTime<Utc>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Invitation status
    pub status: InvitationStatus,
}

/// Invitation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvitationStatus {
    Pending,
    Accepted,
    Declined,
    Expired,
    Cancelled,
}

/// Real-time presence information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceInfo {
    /// User ID
    pub user_id: MemberId,
    /// Display name
    pub display_name: String,
    /// Current session ID (if viewing a session)
    pub current_session: Option<String>,
    /// Current cursor position (for collaborative editing)
    pub cursor_position: Option<CursorPosition>,
    /// User status
    pub status: PresenceStatus,
    /// Last heartbeat
    pub last_heartbeat: DateTime<Utc>,
}

/// Cursor position for collaborative editing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    /// Session ID
    pub session_id: String,
    /// Message index
    pub message_index: usize,
    /// Character offset
    pub offset: usize,
}

/// User presence status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresenceStatus {
    Online,
    Away,
    Busy,
    Offline,
}

/// Real-time collaboration event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CollaborationEvent {
    /// Member joined the session
    MemberJoined {
        user_id: MemberId,
        session_id: String,
    },
    /// Member left the session
    MemberLeft {
        user_id: MemberId,
        session_id: String,
    },
    /// Cursor moved
    CursorMoved {
        user_id: MemberId,
        position: CursorPosition,
    },
    /// Session content changed
    ContentChanged {
        user_id: MemberId,
        session_id: String,
        change_type: ContentChangeType,
    },
    /// Presence updated
    PresenceUpdated {
        user_id: MemberId,
        status: PresenceStatus,
    },
    /// Session shared
    SessionShared {
        user_id: MemberId,
        session_id: String,
    },
    /// Comment added
    CommentAdded {
        user_id: MemberId,
        session_id: String,
        comment_id: String,
    },
}

/// Content change type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentChangeType {
    MessageAdded,
    MessageEdited,
    MessageDeleted,
    TagsUpdated,
    TitleChanged,
    Annotated,
}

// ============================================================================
// Team Manager
// ============================================================================

/// Manager for team workspaces
pub struct TeamManager {
    /// Teams by ID
    teams: Arc<RwLock<HashMap<TeamId, TeamWorkspace>>>,
    /// User's team memberships (user_id -> team_ids)
    user_teams: Arc<RwLock<HashMap<MemberId, Vec<TeamId>>>>,
    /// Active presences by team
    presences: Arc<RwLock<HashMap<TeamId, Vec<PresenceInfo>>>>,
    /// Collaboration event broadcaster
    event_tx: broadcast::Sender<(TeamId, CollaborationEvent)>,
    /// Pending invitations
    invitations: Arc<RwLock<HashMap<Uuid, TeamInvitation>>>,
}

impl TeamManager {
    /// Create a new team manager
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(1000);
        Self {
            teams: Arc::new(RwLock::new(HashMap::new())),
            user_teams: Arc::new(RwLock::new(HashMap::new())),
            presences: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            invitations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new team
    pub async fn create_team(
        &self,
        name: String,
        description: Option<String>,
        owner_id: MemberId,
        owner_name: String,
        owner_email: String,
    ) -> TeamWorkspace {
        let team_id = Uuid::new_v4();
        let now = Utc::now();

        let owner_member = TeamMember {
            user_id: owner_id,
            display_name: owner_name,
            email: owner_email,
            avatar_url: None,
            role: Role::Owner,
            custom_permissions: None,
            joined_at: now,
            last_active: Some(now),
            status: MemberStatus::Active,
        };

        let team = TeamWorkspace {
            id: team_id,
            name,
            description,
            avatar_url: None,
            owner_id,
            settings: TeamSettings::default(),
            members: vec![owner_member],
            shared_sessions: vec![],
            created_at: now,
            updated_at: now,
        };

        // Store team
        self.teams.write().await.insert(team_id, team.clone());

        // Track user's team membership
        self.user_teams
            .write()
            .await
            .entry(owner_id)
            .or_default()
            .push(team_id);

        team
    }

    /// Get a team by ID
    pub async fn get_team(&self, team_id: TeamId) -> Option<TeamWorkspace> {
        self.teams.read().await.get(&team_id).cloned()
    }

    /// Get all teams for a user
    pub async fn get_user_teams(&self, user_id: MemberId) -> Vec<TeamWorkspace> {
        let user_teams = self.user_teams.read().await;
        let teams = self.teams.read().await;

        user_teams
            .get(&user_id)
            .map(|team_ids| {
                team_ids
                    .iter()
                    .filter_map(|id| teams.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Update team settings
    pub async fn update_team_settings(
        &self,
        team_id: TeamId,
        settings: TeamSettings,
    ) -> Option<TeamWorkspace> {
        let mut teams = self.teams.write().await;
        if let Some(team) = teams.get_mut(&team_id) {
            team.settings = settings;
            team.updated_at = Utc::now();
            Some(team.clone())
        } else {
            None
        }
    }

    /// Invite a member to a team
    pub async fn invite_member(
        &self,
        team_id: TeamId,
        inviter_id: MemberId,
        invitee_email: String,
        role: Role,
        message: Option<String>,
    ) -> Option<TeamInvitation> {
        let teams = self.teams.read().await;
        if !teams.contains_key(&team_id) {
            return None;
        }

        let invitation = TeamInvitation {
            id: Uuid::new_v4(),
            team_id,
            inviter_id,
            invitee_email,
            role,
            message,
            expires_at: Utc::now() + chrono::Duration::days(7),
            created_at: Utc::now(),
            status: InvitationStatus::Pending,
        };

        self.invitations
            .write()
            .await
            .insert(invitation.id, invitation.clone());

        Some(invitation)
    }

    /// Accept an invitation
    pub async fn accept_invitation(
        &self,
        invitation_id: Uuid,
        user_id: MemberId,
        display_name: String,
        email: String,
    ) -> Result<TeamWorkspace, String> {
        let mut invitations = self.invitations.write().await;
        let invitation = invitations
            .get_mut(&invitation_id)
            .ok_or("Invitation not found")?;

        if invitation.status != InvitationStatus::Pending {
            return Err("Invitation is no longer pending".to_string());
        }

        if invitation.expires_at < Utc::now() {
            invitation.status = InvitationStatus::Expired;
            return Err("Invitation has expired".to_string());
        }

        invitation.status = InvitationStatus::Accepted;

        // Add member to team
        let mut teams = self.teams.write().await;
        let team = teams
            .get_mut(&invitation.team_id)
            .ok_or("Team not found")?;

        let now = Utc::now();
        let member = TeamMember {
            user_id,
            display_name,
            email,
            avatar_url: None,
            role: invitation.role,
            custom_permissions: None,
            joined_at: now,
            last_active: Some(now),
            status: MemberStatus::Active,
        };

        team.members.push(member);
        team.updated_at = now;

        // Track user's team membership
        self.user_teams
            .write()
            .await
            .entry(user_id)
            .or_default()
            .push(invitation.team_id);

        Ok(team.clone())
    }

    /// Remove a member from a team
    pub async fn remove_member(
        &self,
        team_id: TeamId,
        member_id: MemberId,
    ) -> Result<(), String> {
        let mut teams = self.teams.write().await;
        let team = teams.get_mut(&team_id).ok_or("Team not found")?;

        // Cannot remove owner
        if team.owner_id == member_id {
            return Err("Cannot remove team owner".to_string());
        }

        team.members.retain(|m| m.user_id != member_id);
        team.updated_at = Utc::now();

        // Remove from user's team memberships
        if let Some(user_teams) = self.user_teams.write().await.get_mut(&member_id) {
            user_teams.retain(|id| *id != team_id);
        }

        Ok(())
    }

    /// Update member role
    pub async fn update_member_role(
        &self,
        team_id: TeamId,
        member_id: MemberId,
        new_role: Role,
    ) -> Result<(), String> {
        let mut teams = self.teams.write().await;
        let team = teams.get_mut(&team_id).ok_or("Team not found")?;

        // Cannot change owner's role
        if team.owner_id == member_id && new_role != Role::Owner {
            return Err("Cannot change owner's role".to_string());
        }

        if let Some(member) = team.members.iter_mut().find(|m| m.user_id == member_id) {
            member.role = new_role;
            team.updated_at = Utc::now();
            Ok(())
        } else {
            Err("Member not found".to_string())
        }
    }

    /// Share a session with the team
    pub async fn share_session(
        &self,
        team_id: TeamId,
        session_id: String,
        sharer_id: MemberId,
    ) -> Result<(), String> {
        let mut teams = self.teams.write().await;
        let team = teams.get_mut(&team_id).ok_or("Team not found")?;

        if !team.shared_sessions.contains(&session_id) {
            team.shared_sessions.push(session_id.clone());
            team.updated_at = Utc::now();

            // Broadcast event
            let _ = self.event_tx.send((
                team_id,
                CollaborationEvent::SessionShared {
                    user_id: sharer_id,
                    session_id,
                },
            ));
        }

        Ok(())
    }

    /// Update presence for a user
    pub async fn update_presence(&self, team_id: TeamId, presence: PresenceInfo) {
        let mut presences = self.presences.write().await;
        let team_presences = presences.entry(team_id).or_default();

        // Update or add presence
        if let Some(existing) = team_presences
            .iter_mut()
            .find(|p| p.user_id == presence.user_id)
        {
            *existing = presence.clone();
        } else {
            team_presences.push(presence.clone());
        }

        // Broadcast presence update
        let _ = self.event_tx.send((
            team_id,
            CollaborationEvent::PresenceUpdated {
                user_id: presence.user_id,
                status: presence.status,
            },
        ));
    }

    /// Get active presences for a team
    pub async fn get_presences(&self, team_id: TeamId) -> Vec<PresenceInfo> {
        self.presences
            .read()
            .await
            .get(&team_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Subscribe to collaboration events
    pub fn subscribe(&self) -> broadcast::Receiver<(TeamId, CollaborationEvent)> {
        self.event_tx.subscribe()
    }

    /// Broadcast a collaboration event
    pub async fn broadcast_event(&self, team_id: TeamId, event: CollaborationEvent) {
        let _ = self.event_tx.send((team_id, event));
    }

    /// Delete a team
    pub async fn delete_team(&self, team_id: TeamId, requester_id: MemberId) -> Result<(), String> {
        let teams = self.teams.read().await;
        let team = teams.get(&team_id).ok_or("Team not found")?;

        if team.owner_id != requester_id {
            return Err("Only the owner can delete the team".to_string());
        }

        drop(teams);

        // Remove team
        self.teams.write().await.remove(&team_id);

        // Remove from all user memberships
        let mut user_teams = self.user_teams.write().await;
        for team_ids in user_teams.values_mut() {
            team_ids.retain(|id| *id != team_id);
        }

        // Remove presences
        self.presences.write().await.remove(&team_id);

        Ok(())
    }
}

impl Default for TeamManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_team() {
        let manager = TeamManager::new();
        let owner_id = Uuid::new_v4();

        let team = manager
            .create_team(
                "Test Team".to_string(),
                Some("A test team".to_string()),
                owner_id,
                "Owner".to_string(),
                "owner@example.com".to_string(),
            )
            .await;

        assert_eq!(team.name, "Test Team");
        assert_eq!(team.owner_id, owner_id);
        assert_eq!(team.members.len(), 1);
        assert_eq!(team.members[0].role, Role::Owner);
    }

    #[tokio::test]
    async fn test_invite_and_accept() {
        let manager = TeamManager::new();
        let owner_id = Uuid::new_v4();
        let member_id = Uuid::new_v4();

        let team = manager
            .create_team(
                "Test Team".to_string(),
                None,
                owner_id,
                "Owner".to_string(),
                "owner@example.com".to_string(),
            )
            .await;

        let invitation = manager
            .invite_member(
                team.id,
                owner_id,
                "member@example.com".to_string(),
                Role::Member,
                None,
            )
            .await
            .unwrap();

        let updated_team = manager
            .accept_invitation(
                invitation.id,
                member_id,
                "Member".to_string(),
                "member@example.com".to_string(),
            )
            .await
            .unwrap();

        assert_eq!(updated_team.members.len(), 2);
    }
}
