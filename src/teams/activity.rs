// Copyright (c) 2024-2027 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Team activity feed module
//!
//! Tracks and provides team activities, notifications, and audit trail.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use super::workspace::{MemberId, TeamId};

// ============================================================================
// Activity Types
// ============================================================================

/// Activity event that occurred in a team
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEvent {
    /// Unique event ID
    pub id: Uuid,
    /// Team ID
    pub team_id: TeamId,
    /// Actor who performed the action
    pub actor_id: MemberId,
    /// Actor display name
    pub actor_name: String,
    /// Event type
    pub event_type: EventType,
    /// Event details
    pub details: EventDetails,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// IP address (for audit)
    pub ip_address: Option<String>,
    /// User agent (for audit)
    pub user_agent: Option<String>,
}

/// Type of activity event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    // Team events
    TeamCreated,
    TeamUpdated,
    TeamDeleted,
    SettingsChanged,

    // Member events
    MemberInvited,
    MemberJoined,
    MemberLeft,
    MemberRemoved,
    RoleChanged,

    // Session events
    SessionCreated,
    SessionUpdated,
    SessionDeleted,
    SessionShared,
    SessionArchived,
    SessionExported,

    // Collaboration events
    CommentAdded,
    CommentEdited,
    CommentDeleted,
    AnnotationAdded,

    // Security events
    PermissionGranted,
    PermissionRevoked,
    AccessDenied,
    SuspiciousActivity,

    // Integration events
    WebhookTriggered,
    IntegrationConnected,
    IntegrationDisconnected,
    HarvestCompleted,
}

/// Detailed information about the event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventDetails {
    /// Team-related details
    Team {
        team_name: String,
        changes: Option<HashMap<String, ChangeValue>>,
    },
    /// Member-related details
    Member {
        member_id: MemberId,
        member_name: String,
        member_email: Option<String>,
        role: Option<String>,
        previous_role: Option<String>,
    },
    /// Session-related details
    Session {
        session_id: String,
        session_title: String,
        provider: Option<String>,
    },
    /// Comment-related details
    Comment {
        comment_id: String,
        session_id: String,
        content_preview: Option<String>,
    },
    /// Permission-related details
    Permission {
        permission: String,
        target_id: Option<String>,
        target_type: Option<String>,
    },
    /// Integration-related details
    Integration {
        integration_name: String,
        integration_type: String,
    },
    /// Generic details
    Generic {
        message: String,
        metadata: Option<HashMap<String, String>>,
    },
}

/// Value change for audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeValue {
    pub old: Option<String>,
    pub new: Option<String>,
}

// ============================================================================
// Notifications
// ============================================================================

/// Notification for a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    /// Notification ID
    pub id: Uuid,
    /// Recipient user ID
    pub user_id: MemberId,
    /// Team ID
    pub team_id: TeamId,
    /// Notification type
    pub notification_type: NotificationType,
    /// Notification title
    pub title: String,
    /// Notification message
    pub message: String,
    /// Related event ID
    pub event_id: Option<Uuid>,
    /// Action URL
    pub action_url: Option<String>,
    /// Read status
    pub read: bool,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Read timestamp
    pub read_at: Option<DateTime<Utc>>,
}

/// Type of notification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    /// Team invitation
    Invitation,
    /// Mention in comment
    Mention,
    /// Session shared with user
    SessionShare,
    /// Comment on user's session
    Comment,
    /// Role changed
    RoleChange,
    /// Security alert
    SecurityAlert,
    /// System announcement
    Announcement,
}

/// Notification preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreferences {
    /// User ID
    pub user_id: MemberId,
    /// Email notifications enabled
    pub email_enabled: bool,
    /// Push notifications enabled
    pub push_enabled: bool,
    /// In-app notifications enabled
    pub in_app_enabled: bool,
    /// Notification types to receive
    pub enabled_types: HashMap<NotificationType, bool>,
    /// Quiet hours start (24h format)
    pub quiet_hours_start: Option<u8>,
    /// Quiet hours end (24h format)
    pub quiet_hours_end: Option<u8>,
}

impl Default for NotificationPreferences {
    fn default() -> Self {
        let mut enabled_types = HashMap::new();
        enabled_types.insert(NotificationType::Invitation, true);
        enabled_types.insert(NotificationType::Mention, true);
        enabled_types.insert(NotificationType::SessionShare, true);
        enabled_types.insert(NotificationType::Comment, true);
        enabled_types.insert(NotificationType::RoleChange, true);
        enabled_types.insert(NotificationType::SecurityAlert, true);
        enabled_types.insert(NotificationType::Announcement, true);

        Self {
            user_id: Uuid::nil(),
            email_enabled: true,
            push_enabled: true,
            in_app_enabled: true,
            enabled_types,
            quiet_hours_start: None,
            quiet_hours_end: None,
        }
    }
}

// ============================================================================
// Activity Manager
// ============================================================================

/// Manager for team activity tracking
pub struct ActivityManager {
    /// Activity events by team (ring buffer per team)
    activities: Arc<RwLock<HashMap<TeamId, VecDeque<ActivityEvent>>>>,
    /// Maximum events per team
    max_events_per_team: usize,
    /// Event broadcaster
    event_tx: broadcast::Sender<ActivityEvent>,
    /// User notifications
    notifications: Arc<RwLock<HashMap<MemberId, Vec<Notification>>>>,
    /// Notification preferences
    preferences: Arc<RwLock<HashMap<MemberId, NotificationPreferences>>>,
}

impl ActivityManager {
    /// Create a new activity manager
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(1000);
        Self {
            activities: Arc::new(RwLock::new(HashMap::new())),
            max_events_per_team: 10000,
            event_tx,
            notifications: Arc::new(RwLock::new(HashMap::new())),
            preferences: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record an activity event
    pub async fn record_event(&self, event: ActivityEvent) {
        let team_id = event.team_id;

        // Store event
        let mut activities = self.activities.write().await;
        let team_events = activities.entry(team_id).or_insert_with(VecDeque::new);

        // Maintain ring buffer
        if team_events.len() >= self.max_events_per_team {
            team_events.pop_front();
        }
        team_events.push_back(event.clone());

        // Broadcast event
        let _ = self.event_tx.send(event);
    }

    /// Get recent activities for a team
    pub async fn get_activities(
        &self,
        team_id: TeamId,
        limit: usize,
        offset: usize,
    ) -> Vec<ActivityEvent> {
        let activities = self.activities.read().await;
        activities
            .get(&team_id)
            .map(|events| {
                events
                    .iter()
                    .rev()
                    .skip(offset)
                    .take(limit)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get activities by type
    pub async fn get_activities_by_type(
        &self,
        team_id: TeamId,
        event_type: EventType,
        limit: usize,
    ) -> Vec<ActivityEvent> {
        let activities = self.activities.read().await;
        activities
            .get(&team_id)
            .map(|events| {
                events
                    .iter()
                    .rev()
                    .filter(|e| e.event_type == event_type)
                    .take(limit)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get activities by actor
    pub async fn get_activities_by_actor(
        &self,
        team_id: TeamId,
        actor_id: MemberId,
        limit: usize,
    ) -> Vec<ActivityEvent> {
        let activities = self.activities.read().await;
        activities
            .get(&team_id)
            .map(|events| {
                events
                    .iter()
                    .rev()
                    .filter(|e| e.actor_id == actor_id)
                    .take(limit)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get activities in a time range
    pub async fn get_activities_in_range(
        &self,
        team_id: TeamId,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<ActivityEvent> {
        let activities = self.activities.read().await;
        activities
            .get(&team_id)
            .map(|events| {
                events
                    .iter()
                    .filter(|e| e.timestamp >= start && e.timestamp <= end)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Subscribe to activity events
    pub fn subscribe(&self) -> broadcast::Receiver<ActivityEvent> {
        self.event_tx.subscribe()
    }

    /// Create a notification for a user
    pub async fn create_notification(&self, notification: Notification) {
        let user_id = notification.user_id;

        // Check preferences
        let prefs = self.preferences.read().await;
        if let Some(user_prefs) = prefs.get(&user_id) {
            if !user_prefs.in_app_enabled {
                return;
            }
            if let Some(enabled) = user_prefs
                .enabled_types
                .get(&notification.notification_type)
            {
                if !enabled {
                    return;
                }
            }
        }

        // Store notification
        let mut notifications = self.notifications.write().await;
        notifications.entry(user_id).or_default().push(notification);
    }

    /// Get unread notifications for a user
    pub async fn get_unread_notifications(&self, user_id: MemberId) -> Vec<Notification> {
        let notifications = self.notifications.read().await;
        notifications
            .get(&user_id)
            .map(|notifs| notifs.iter().filter(|n| !n.read).cloned().collect())
            .unwrap_or_default()
    }

    /// Get all notifications for a user
    pub async fn get_notifications(
        &self,
        user_id: MemberId,
        limit: usize,
        offset: usize,
    ) -> Vec<Notification> {
        let notifications = self.notifications.read().await;
        notifications
            .get(&user_id)
            .map(|notifs| {
                notifs
                    .iter()
                    .rev()
                    .skip(offset)
                    .take(limit)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Mark notification as read
    pub async fn mark_as_read(&self, user_id: MemberId, notification_id: Uuid) {
        let mut notifications = self.notifications.write().await;
        if let Some(user_notifs) = notifications.get_mut(&user_id) {
            if let Some(notif) = user_notifs.iter_mut().find(|n| n.id == notification_id) {
                notif.read = true;
                notif.read_at = Some(Utc::now());
            }
        }
    }

    /// Mark all notifications as read for a user
    pub async fn mark_all_as_read(&self, user_id: MemberId) {
        let mut notifications = self.notifications.write().await;
        if let Some(user_notifs) = notifications.get_mut(&user_id) {
            let now = Utc::now();
            for notif in user_notifs.iter_mut() {
                if !notif.read {
                    notif.read = true;
                    notif.read_at = Some(now);
                }
            }
        }
    }

    /// Delete a notification
    pub async fn delete_notification(&self, user_id: MemberId, notification_id: Uuid) {
        let mut notifications = self.notifications.write().await;
        if let Some(user_notifs) = notifications.get_mut(&user_id) {
            user_notifs.retain(|n| n.id != notification_id);
        }
    }

    /// Update notification preferences
    pub async fn update_preferences(&self, preferences: NotificationPreferences) {
        self.preferences
            .write()
            .await
            .insert(preferences.user_id, preferences);
    }

    /// Get notification preferences for a user
    pub async fn get_preferences(&self, user_id: MemberId) -> NotificationPreferences {
        self.preferences
            .read()
            .await
            .get(&user_id)
            .cloned()
            .unwrap_or_else(|| {
                let mut prefs = NotificationPreferences::default();
                prefs.user_id = user_id;
                prefs
            })
    }

    /// Get unread notification count for a user
    pub async fn get_unread_count(&self, user_id: MemberId) -> usize {
        let notifications = self.notifications.read().await;
        notifications
            .get(&user_id)
            .map(|notifs| notifs.iter().filter(|n| !n.read).count())
            .unwrap_or(0)
    }
}

impl Default for ActivityManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a team activity event
pub fn team_event(
    team_id: TeamId,
    actor_id: MemberId,
    actor_name: String,
    event_type: EventType,
    team_name: String,
    changes: Option<HashMap<String, ChangeValue>>,
) -> ActivityEvent {
    ActivityEvent {
        id: Uuid::new_v4(),
        team_id,
        actor_id,
        actor_name,
        event_type,
        details: EventDetails::Team { team_name, changes },
        timestamp: Utc::now(),
        ip_address: None,
        user_agent: None,
    }
}

/// Create a member activity event
pub fn member_event(
    team_id: TeamId,
    actor_id: MemberId,
    actor_name: String,
    event_type: EventType,
    member_id: MemberId,
    member_name: String,
    member_email: Option<String>,
    role: Option<String>,
    previous_role: Option<String>,
) -> ActivityEvent {
    ActivityEvent {
        id: Uuid::new_v4(),
        team_id,
        actor_id,
        actor_name,
        event_type,
        details: EventDetails::Member {
            member_id,
            member_name,
            member_email,
            role,
            previous_role,
        },
        timestamp: Utc::now(),
        ip_address: None,
        user_agent: None,
    }
}

/// Create a session activity event
pub fn session_event(
    team_id: TeamId,
    actor_id: MemberId,
    actor_name: String,
    event_type: EventType,
    session_id: String,
    session_title: String,
    provider: Option<String>,
) -> ActivityEvent {
    ActivityEvent {
        id: Uuid::new_v4(),
        team_id,
        actor_id,
        actor_name,
        event_type,
        details: EventDetails::Session {
            session_id,
            session_title,
            provider,
        },
        timestamp: Utc::now(),
        ip_address: None,
        user_agent: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_and_get_activities() {
        let manager = ActivityManager::new();
        let team_id = Uuid::new_v4();
        let actor_id = Uuid::new_v4();

        let event = team_event(
            team_id,
            actor_id,
            "Test User".to_string(),
            EventType::TeamCreated,
            "Test Team".to_string(),
            None,
        );

        manager.record_event(event.clone()).await;

        let activities = manager.get_activities(team_id, 10, 0).await;
        assert_eq!(activities.len(), 1);
        assert_eq!(activities[0].event_type, EventType::TeamCreated);
    }

    #[tokio::test]
    async fn test_notifications() {
        let manager = ActivityManager::new();
        let user_id = Uuid::new_v4();
        let team_id = Uuid::new_v4();

        let notification = Notification {
            id: Uuid::new_v4(),
            user_id,
            team_id,
            notification_type: NotificationType::Invitation,
            title: "Team Invitation".to_string(),
            message: "You have been invited to join a team".to_string(),
            event_id: None,
            action_url: None,
            read: false,
            created_at: Utc::now(),
            read_at: None,
        };

        manager.create_notification(notification.clone()).await;

        let unread = manager.get_unread_notifications(user_id).await;
        assert_eq!(unread.len(), 1);

        manager.mark_as_read(user_id, notification.id).await;

        let unread = manager.get_unread_notifications(user_id).await;
        assert_eq!(unread.len(), 0);
    }
}
