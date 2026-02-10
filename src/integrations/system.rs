// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! System Integrations
//!
//! Shell, Clipboard, Filesystem, Notifications, System Info

use super::IntegrationResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// =============================================================================
// Shell / Terminal
// =============================================================================

/// Shell command result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub command: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub started_at: String,
}

/// Shell environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellEnvironment {
    pub shell: String,
    pub cwd: PathBuf,
    pub env_vars: HashMap<String, String>,
}

/// Shell provider trait
#[async_trait::async_trait]
pub trait ShellProvider: Send + Sync {
    /// Run a command and wait for result
    async fn run_command(&self, command: &str, cwd: Option<&str>) -> IntegrationResult;

    /// Run a command with custom environment
    async fn run_with_env(&self, command: &str, env: HashMap<String, String>) -> IntegrationResult;

    /// Run a command in the background
    async fn run_background(&self, command: &str) -> IntegrationResult;

    /// Kill a background process
    async fn kill_process(&self, pid: u32) -> IntegrationResult;

    /// Get current shell environment
    async fn get_environment(&self) -> IntegrationResult;

    /// Set environment variable
    async fn set_env_var(&self, name: &str, value: &str) -> IntegrationResult;

    /// Source a shell script
    async fn source_script(&self, path: &str) -> IntegrationResult;
}

// =============================================================================
// Clipboard
// =============================================================================

/// Clipboard content types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClipboardContent {
    Text(String),
    Html(String),
    Image(Vec<u8>),
    Files(Vec<PathBuf>),
    RichText { text: String, rtf: String },
}

/// Clipboard history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardEntry {
    pub content: ClipboardContent,
    pub timestamp: String,
    pub source_app: Option<String>,
}

/// Clipboard provider trait
#[async_trait::async_trait]
pub trait ClipboardProvider: Send + Sync {
    /// Get current clipboard content
    async fn get(&self) -> IntegrationResult;

    /// Set clipboard content
    async fn set_text(&self, text: &str) -> IntegrationResult;

    /// Set clipboard HTML
    async fn set_html(&self, html: &str) -> IntegrationResult;

    /// Set clipboard image
    async fn set_image(&self, image_data: Vec<u8>) -> IntegrationResult;

    /// Get clipboard history (requires clipboard manager)
    async fn get_history(&self, limit: u32) -> IntegrationResult;

    /// Clear clipboard
    async fn clear(&self) -> IntegrationResult;
}

// =============================================================================
// Filesystem
// =============================================================================

/// File info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub is_file: bool,
    pub is_symlink: bool,
    pub size_bytes: u64,
    pub created: Option<String>,
    pub modified: Option<String>,
    pub accessed: Option<String>,
    pub permissions: Option<u32>,
}

/// File search options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchOptions {
    pub pattern: String,
    pub recursive: bool,
    pub include_hidden: bool,
    pub file_types: Option<Vec<String>>,
    pub max_depth: Option<u32>,
    pub max_results: Option<u32>,
}

/// Watch event types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchEvent {
    Created,
    Modified,
    Deleted,
    Renamed,
    Accessed,
}

/// Filesystem provider trait
#[async_trait::async_trait]
pub trait FilesystemProvider: Send + Sync {
    // Reading
    async fn read_file(&self, path: &str) -> IntegrationResult;
    async fn read_file_bytes(&self, path: &str) -> IntegrationResult;
    async fn read_json(&self, path: &str) -> IntegrationResult;
    async fn list_dir(&self, path: &str) -> IntegrationResult;
    async fn get_file_info(&self, path: &str) -> IntegrationResult;

    // Writing
    async fn write_file(&self, path: &str, content: &str) -> IntegrationResult;
    async fn write_file_bytes(&self, path: &str, content: Vec<u8>) -> IntegrationResult;
    async fn append_file(&self, path: &str, content: &str) -> IntegrationResult;

    // Operations
    async fn create_dir(&self, path: &str, recursive: bool) -> IntegrationResult;
    async fn copy(&self, src: &str, dst: &str) -> IntegrationResult;
    async fn move_path(&self, src: &str, dst: &str) -> IntegrationResult;
    async fn delete(&self, path: &str, recursive: bool) -> IntegrationResult;
    async fn exists(&self, path: &str) -> IntegrationResult;

    // Search
    async fn search(&self, base_path: &str, options: SearchOptions) -> IntegrationResult;
    async fn glob(&self, pattern: &str) -> IntegrationResult;

    // Watch
    async fn watch(&self, path: &str, events: Vec<WatchEvent>) -> IntegrationResult;
    async fn unwatch(&self, watch_id: &str) -> IntegrationResult;
}

// =============================================================================
// System Notifications
// =============================================================================

/// Notification priority
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationPriority {
    Low,
    Normal,
    High,
    Urgent,
}

/// Notification action button
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationAction {
    pub id: String,
    pub label: String,
    pub is_destructive: bool,
}

/// Notification options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NotificationOptions {
    pub title: String,
    pub body: String,
    pub subtitle: Option<String>,
    pub icon: Option<String>,
    pub image: Option<String>,
    pub sound: Option<String>,
    pub priority: Option<NotificationPriority>,
    pub actions: Vec<NotificationAction>,
    pub timeout_ms: Option<u32>,
    pub silent: bool,
}

/// System notifications provider trait
#[async_trait::async_trait]
pub trait SystemNotificationProvider: Send + Sync {
    /// Send a notification
    async fn notify(&self, options: NotificationOptions) -> IntegrationResult;

    /// Schedule a notification
    async fn schedule(&self, options: NotificationOptions, at: &str) -> IntegrationResult;

    /// Cancel a scheduled notification
    async fn cancel(&self, notification_id: &str) -> IntegrationResult;

    /// List pending notifications
    async fn list_pending(&self) -> IntegrationResult;

    /// Request notification permission
    async fn request_permission(&self) -> IntegrationResult;
}

// =============================================================================
// System Info
// =============================================================================

/// CPU info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    pub model: String,
    pub cores: u32,
    pub threads: u32,
    pub usage_percent: f32,
    pub frequency_mhz: u64,
}

/// Memory info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub available_bytes: u64,
    pub usage_percent: f32,
}

/// Disk info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub fs_type: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub usage_percent: f32,
}

/// Network interface info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub name: String,
    pub ip_address: Option<String>,
    pub mac_address: Option<String>,
    pub is_up: bool,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

/// Battery info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryInfo {
    pub charge_percent: u8,
    pub is_charging: bool,
    pub is_plugged_in: bool,
    pub time_to_full_mins: Option<u32>,
    pub time_to_empty_mins: Option<u32>,
    pub health_percent: Option<u8>,
}

/// System info provider trait
#[async_trait::async_trait]
pub trait SystemInfoProvider: Send + Sync {
    async fn get_os_info(&self) -> IntegrationResult;
    async fn get_cpu_info(&self) -> IntegrationResult;
    async fn get_memory_info(&self) -> IntegrationResult;
    async fn get_disk_info(&self) -> IntegrationResult;
    async fn get_network_info(&self) -> IntegrationResult;
    async fn get_battery_info(&self) -> IntegrationResult;
    async fn get_uptime(&self) -> IntegrationResult;
    async fn get_load_average(&self) -> IntegrationResult;
    async fn get_processes(&self, limit: Option<u32>) -> IntegrationResult;
}

// =============================================================================
// Application Control
// =============================================================================

/// Application info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub bundle_id: Option<String>,
    pub path: PathBuf,
    pub version: Option<String>,
    pub is_running: bool,
    pub pid: Option<u32>,
}

/// Window info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub id: u64,
    pub title: String,
    pub app_name: String,
    pub is_focused: bool,
    pub is_visible: bool,
    pub position: (i32, i32),
    pub size: (u32, u32),
}

/// Application control provider trait
#[async_trait::async_trait]
pub trait AppControlProvider: Send + Sync {
    // Apps
    async fn list_installed_apps(&self) -> IntegrationResult;
    async fn list_running_apps(&self) -> IntegrationResult;
    async fn launch_app(&self, app_id: &str) -> IntegrationResult;
    async fn quit_app(&self, app_id: &str) -> IntegrationResult;
    async fn focus_app(&self, app_id: &str) -> IntegrationResult;

    // Windows
    async fn list_windows(&self) -> IntegrationResult;
    async fn focus_window(&self, window_id: u64) -> IntegrationResult;
    async fn close_window(&self, window_id: u64) -> IntegrationResult;
    async fn minimize_window(&self, window_id: u64) -> IntegrationResult;
    async fn maximize_window(&self, window_id: u64) -> IntegrationResult;
    async fn move_window(&self, window_id: u64, x: i32, y: i32) -> IntegrationResult;
    async fn resize_window(&self, window_id: u64, width: u32, height: u32) -> IntegrationResult;
}

// =============================================================================
// Keyboard / Input Simulation
// =============================================================================

/// Key modifier
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyModifier {
    Shift,
    Ctrl,
    Alt,
    Meta,
    Cmd,
}

/// Input provider trait
#[async_trait::async_trait]
pub trait InputProvider: Send + Sync {
    /// Type text
    async fn type_text(&self, text: &str, delay_ms: Option<u32>) -> IntegrationResult;

    /// Press a key
    async fn press_key(&self, key: &str, modifiers: Vec<KeyModifier>) -> IntegrationResult;

    /// Press a key combination
    async fn key_combo(&self, keys: Vec<&str>) -> IntegrationResult;

    /// Move mouse
    async fn move_mouse(&self, x: i32, y: i32) -> IntegrationResult;

    /// Click mouse
    async fn click(&self, button: &str) -> IntegrationResult;

    /// Double click
    async fn double_click(&self) -> IntegrationResult;

    /// Scroll
    async fn scroll(&self, dx: i32, dy: i32) -> IntegrationResult;
}

// =============================================================================
// Audio Control
// =============================================================================

/// Audio device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub device_type: AudioDeviceType,
    pub is_default: bool,
    pub volume: u8,
    pub is_muted: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioDeviceType {
    Output,
    Input,
}

/// Audio control provider trait
#[async_trait::async_trait]
pub trait AudioProvider: Send + Sync {
    async fn list_devices(&self) -> IntegrationResult;
    async fn get_volume(&self) -> IntegrationResult;
    async fn set_volume(&self, volume: u8) -> IntegrationResult;
    async fn mute(&self) -> IntegrationResult;
    async fn unmute(&self) -> IntegrationResult;
    async fn set_default_device(&self, device_id: &str) -> IntegrationResult;
}

// =============================================================================
// Display Control
// =============================================================================

/// Display info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayInfo {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub refresh_rate: u32,
    pub is_primary: bool,
    pub brightness: Option<u8>,
    pub scale_factor: f32,
}

/// Display control provider trait
#[async_trait::async_trait]
pub trait DisplayProvider: Send + Sync {
    async fn list_displays(&self) -> IntegrationResult;
    async fn get_brightness(&self, display_id: &str) -> IntegrationResult;
    async fn set_brightness(&self, display_id: &str, brightness: u8) -> IntegrationResult;
    async fn take_screenshot(&self, display_id: Option<&str>) -> IntegrationResult;
    async fn take_screenshot_region(
        &self,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> IntegrationResult;
}

// =============================================================================
// Power Management
// =============================================================================

/// Power action
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerAction {
    Shutdown,
    Restart,
    Sleep,
    Hibernate,
    Lock,
    LogOut,
}

/// Power management provider trait
#[async_trait::async_trait]
pub trait PowerProvider: Send + Sync {
    async fn perform_action(&self, action: PowerAction) -> IntegrationResult;
    async fn schedule_action(&self, action: PowerAction, at: &str) -> IntegrationResult;
    async fn cancel_scheduled(&self) -> IntegrationResult;
    async fn prevent_sleep(&self, reason: &str) -> IntegrationResult;
    async fn allow_sleep(&self) -> IntegrationResult;
}

// =============================================================================
// Cron / Scheduled Tasks
// =============================================================================

/// Scheduled task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: String,
    pub name: String,
    pub command: String,
    pub schedule: String, // cron expression
    pub enabled: bool,
    pub last_run: Option<String>,
    pub next_run: Option<String>,
}

/// Scheduler provider trait
#[async_trait::async_trait]
pub trait SchedulerProvider: Send + Sync {
    async fn list_tasks(&self) -> IntegrationResult;
    async fn create_task(&self, name: &str, command: &str, schedule: &str) -> IntegrationResult;
    async fn update_task(
        &self,
        task_id: &str,
        command: Option<&str>,
        schedule: Option<&str>,
    ) -> IntegrationResult;
    async fn delete_task(&self, task_id: &str) -> IntegrationResult;
    async fn enable_task(&self, task_id: &str) -> IntegrationResult;
    async fn disable_task(&self, task_id: &str) -> IntegrationResult;
    async fn run_task_now(&self, task_id: &str) -> IntegrationResult;
}
