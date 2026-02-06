// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Plugin System Module
//!
//! Provides an extensible plugin architecture for CSM:
//! - Plugin discovery and loading
//! - Plugin lifecycle management
//! - Event hooks and callbacks
//! - Configuration management
//! - Sandboxed execution

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

// =============================================================================
// Plugin Manifest and Metadata
// =============================================================================

/// Plugin manifest (plugin.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin unique identifier
    pub id: String,
    /// Plugin name
    pub name: String,
    /// Plugin version (semver)
    pub version: String,
    /// Plugin description
    pub description: Option<String>,
    /// Author information
    pub author: Option<PluginAuthor>,
    /// Plugin homepage URL
    pub homepage: Option<String>,
    /// Repository URL
    pub repository: Option<String>,
    /// License identifier
    pub license: Option<String>,
    /// Minimum CSM version required
    pub csm_version: String,
    /// Plugin entry point
    pub main: String,
    /// Required permissions
    pub permissions: Vec<Permission>,
    /// Event hooks this plugin registers
    pub hooks: Vec<String>,
    /// Configuration schema
    pub config_schema: Option<serde_json::Value>,
    /// Dependencies on other plugins
    pub dependencies: Vec<PluginDependency>,
    /// Plugin category
    pub category: PluginCategory,
    /// Keywords for discovery
    pub keywords: Vec<String>,
}

/// Plugin author information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAuthor {
    pub name: String,
    pub email: Option<String>,
    pub url: Option<String>,
}

/// Plugin dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    /// Dependency plugin ID
    pub id: String,
    /// Version requirement (semver range)
    pub version: String,
    /// Whether dependency is optional
    pub optional: bool,
}

/// Plugin category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCategory {
    /// Provider integration
    Provider,
    /// Export format
    Export,
    /// Analysis/insights
    Analysis,
    /// UI enhancement
    Ui,
    /// Automation
    Automation,
    /// Storage backend
    Storage,
    /// Authentication
    Auth,
    /// Other
    Other,
}

/// Plugin permission
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    /// Read session data
    SessionRead,
    /// Write/modify session data
    SessionWrite,
    /// Delete sessions
    SessionDelete,
    /// Read configuration
    ConfigRead,
    /// Write configuration
    ConfigWrite,
    /// Network access
    Network,
    /// File system access (within plugin directory)
    FileSystem,
    /// Execute shell commands (restricted)
    Shell,
    /// Access sensitive data (encryption keys, etc.)
    Sensitive,
    /// Background execution
    Background,
    /// System notifications
    Notifications,
}

// =============================================================================
// Plugin Instance and State
// =============================================================================

/// Plugin state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginState {
    /// Plugin is loaded but not activated
    Loaded,
    /// Plugin is active and running
    Active,
    /// Plugin is disabled
    Disabled,
    /// Plugin encountered an error
    Error,
    /// Plugin is being updated
    Updating,
}

/// Plugin instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInstance {
    /// Plugin manifest
    pub manifest: PluginManifest,
    /// Current state
    pub state: PluginState,
    /// Installation path
    pub path: PathBuf,
    /// Installed at timestamp
    pub installed_at: DateTime<Utc>,
    /// Last activated timestamp
    pub last_activated: Option<DateTime<Utc>>,
    /// Plugin configuration
    pub config: serde_json::Value,
    /// Error message if in error state
    pub error: Option<String>,
    /// Usage statistics
    pub stats: PluginStats,
}

/// Plugin usage statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginStats {
    /// Number of times activated
    pub activation_count: u64,
    /// Total execution time (milliseconds)
    pub total_execution_ms: u64,
    /// Number of errors
    pub error_count: u64,
    /// Last error timestamp
    pub last_error: Option<DateTime<Utc>>,
}

// =============================================================================
// Plugin Events and Hooks
// =============================================================================

/// Event that plugins can hook into
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginEvent {
    /// Session created
    SessionCreated { session_id: String },
    /// Session updated
    SessionUpdated { session_id: String },
    /// Session deleted
    SessionDeleted { session_id: String },
    /// Session imported
    SessionImported { session_id: String, provider: String },
    /// Session exported
    SessionExported { session_id: String, format: String },
    /// Harvest completed
    HarvestCompleted { session_count: usize, provider: String },
    /// Sync completed
    SyncCompleted { direction: String, changes: usize },
    /// User action
    UserAction { action: String, context: serde_json::Value },
    /// Application startup
    AppStartup,
    /// Application shutdown
    AppShutdown,
    /// Configuration changed
    ConfigChanged { key: String },
    /// Custom event
    Custom { name: String, data: serde_json::Value },
}

/// Hook registration
#[derive(Debug, Clone)]
pub struct HookRegistration {
    /// Plugin ID
    pub plugin_id: String,
    /// Event type pattern
    pub event_pattern: String,
    /// Priority (lower = higher priority)
    pub priority: i32,
    /// Handler ID
    pub handler_id: String,
}

/// Result of a hook invocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResult {
    /// Plugin ID
    pub plugin_id: String,
    /// Success status
    pub success: bool,
    /// Result data
    pub data: Option<serde_json::Value>,
    /// Error message if failed
    pub error: Option<String>,
    /// Execution time (ms)
    pub execution_ms: u64,
}

// =============================================================================
// Plugin API Context
// =============================================================================

/// API context provided to plugins
pub struct PluginContext {
    /// Plugin ID
    pub plugin_id: String,
    /// Granted permissions
    pub permissions: Vec<Permission>,
    /// Plugin data directory
    pub data_dir: PathBuf,
    /// Plugin configuration
    pub config: serde_json::Value,
}

impl PluginContext {
    /// Check if plugin has a permission
    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.contains(permission)
    }

    /// Get plugin data path
    pub fn get_data_path(&self, filename: &str) -> PathBuf {
        self.data_dir.join(filename)
    }

    /// Read plugin data file
    pub fn read_data(&self, filename: &str) -> Result<String> {
        if !self.has_permission(&Permission::FileSystem) {
            return Err(anyhow!("Permission denied: FileSystem"));
        }
        let path = self.get_data_path(filename);
        Ok(std::fs::read_to_string(path)?)
    }

    /// Write plugin data file
    pub fn write_data(&self, filename: &str, content: &str) -> Result<()> {
        if !self.has_permission(&Permission::FileSystem) {
            return Err(anyhow!("Permission denied: FileSystem"));
        }
        std::fs::create_dir_all(&self.data_dir)?;
        let path = self.get_data_path(filename);
        Ok(std::fs::write(path, content)?)
    }
}

// =============================================================================
// Plugin Manager
// =============================================================================

/// Plugin manager handles plugin lifecycle
pub struct PluginManager {
    /// Plugins directory
    plugins_dir: PathBuf,
    /// Loaded plugins
    plugins: Arc<RwLock<HashMap<String, PluginInstance>>>,
    /// Registered hooks
    hooks: Arc<RwLock<Vec<HookRegistration>>>,
    /// Plugin configurations
    configs: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new(plugins_dir: PathBuf) -> Self {
        Self {
            plugins_dir,
            plugins: Arc::new(RwLock::new(HashMap::new())),
            hooks: Arc::new(RwLock::new(Vec::new())),
            configs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize the plugin manager
    pub async fn init(&self) -> Result<()> {
        std::fs::create_dir_all(&self.plugins_dir)?;
        self.discover_plugins().await?;
        Ok(())
    }

    /// Discover and load plugins
    pub async fn discover_plugins(&self) -> Result<Vec<String>> {
        let mut discovered = Vec::new();
        
        if !self.plugins_dir.exists() {
            return Ok(discovered);
        }
        
        for entry in std::fs::read_dir(&self.plugins_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                let manifest_path = path.join("plugin.json");
                if manifest_path.exists() {
                    match self.load_plugin(&path).await {
                        Ok(plugin_id) => discovered.push(plugin_id),
                        Err(e) => {
                            log::warn!("Failed to load plugin at {:?}: {}", path, e);
                        }
                    }
                }
            }
        }
        
        Ok(discovered)
    }

    /// Load a plugin from a directory
    pub async fn load_plugin(&self, plugin_path: &PathBuf) -> Result<String> {
        let manifest_path = plugin_path.join("plugin.json");
        let manifest_content = std::fs::read_to_string(&manifest_path)?;
        let manifest: PluginManifest = serde_json::from_str(&manifest_content)?;
        
        // Validate manifest
        self.validate_manifest(&manifest)?;
        
        // Check dependencies
        self.check_dependencies(&manifest).await?;
        
        let instance = PluginInstance {
            manifest: manifest.clone(),
            state: PluginState::Loaded,
            path: plugin_path.clone(),
            installed_at: Utc::now(),
            last_activated: None,
            config: serde_json::Value::Object(serde_json::Map::new()),
            error: None,
            stats: PluginStats::default(),
        };
        
        let plugin_id = manifest.id.clone();
        self.plugins.write().await.insert(plugin_id.clone(), instance);
        
        log::info!("Loaded plugin: {} v{}", manifest.name, manifest.version);
        Ok(plugin_id)
    }

    /// Validate plugin manifest
    fn validate_manifest(&self, manifest: &PluginManifest) -> Result<()> {
        // Validate ID format
        if manifest.id.is_empty() || manifest.id.len() > 64 {
            return Err(anyhow!("Invalid plugin ID"));
        }
        
        // Validate version (semver)
        if semver::Version::parse(&manifest.version).is_err() {
            return Err(anyhow!("Invalid version format: {}", manifest.version));
        }
        
        // Validate CSM version requirement
        let current_version = env!("CARGO_PKG_VERSION");
        let req = semver::VersionReq::parse(&manifest.csm_version)
            .map_err(|_| anyhow!("Invalid csm_version: {}", manifest.csm_version))?;
        let current = semver::Version::parse(current_version)?;
        
        if !req.matches(&current) {
            return Err(anyhow!(
                "Plugin requires CSM {}, but current version is {}",
                manifest.csm_version,
                current_version
            ));
        }
        
        Ok(())
    }

    /// Check plugin dependencies
    async fn check_dependencies(&self, manifest: &PluginManifest) -> Result<()> {
        let plugins = self.plugins.read().await;
        
        for dep in &manifest.dependencies {
            if dep.optional {
                continue;
            }
            
            let plugin = plugins.get(&dep.id);
            match plugin {
                None => {
                    return Err(anyhow!("Missing required dependency: {}", dep.id));
                }
                Some(p) => {
                    let req = semver::VersionReq::parse(&dep.version)?;
                    let ver = semver::Version::parse(&p.manifest.version)?;
                    if !req.matches(&ver) {
                        return Err(anyhow!(
                            "Dependency {} version {} does not match requirement {}",
                            dep.id, p.manifest.version, dep.version
                        ));
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Activate a plugin
    pub async fn activate(&self, plugin_id: &str) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        let plugin = plugins.get_mut(plugin_id)
            .ok_or_else(|| anyhow!("Plugin not found: {}", plugin_id))?;
        
        if plugin.state == PluginState::Active {
            return Ok(());
        }
        
        // Register hooks
        for hook in &plugin.manifest.hooks {
            self.register_hook(plugin_id, hook, 0).await?;
        }
        
        plugin.state = PluginState::Active;
        plugin.last_activated = Some(Utc::now());
        plugin.stats.activation_count += 1;
        
        log::info!("Activated plugin: {}", plugin_id);
        Ok(())
    }

    /// Deactivate a plugin
    pub async fn deactivate(&self, plugin_id: &str) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        let plugin = plugins.get_mut(plugin_id)
            .ok_or_else(|| anyhow!("Plugin not found: {}", plugin_id))?;
        
        // Unregister hooks
        self.unregister_hooks(plugin_id).await?;
        
        plugin.state = PluginState::Disabled;
        
        log::info!("Deactivated plugin: {}", plugin_id);
        Ok(())
    }

    /// Uninstall a plugin
    pub async fn uninstall(&self, plugin_id: &str) -> Result<()> {
        // Deactivate first
        self.deactivate(plugin_id).await.ok();
        
        let mut plugins = self.plugins.write().await;
        let plugin = plugins.remove(plugin_id)
            .ok_or_else(|| anyhow!("Plugin not found: {}", plugin_id))?;
        
        // Remove plugin directory
        if plugin.path.exists() {
            std::fs::remove_dir_all(&plugin.path)?;
        }
        
        log::info!("Uninstalled plugin: {}", plugin_id);
        Ok(())
    }

    /// Register a hook
    async fn register_hook(&self, plugin_id: &str, event_pattern: &str, priority: i32) -> Result<()> {
        let mut hooks = self.hooks.write().await;
        hooks.push(HookRegistration {
            plugin_id: plugin_id.to_string(),
            event_pattern: event_pattern.to_string(),
            priority,
            handler_id: uuid::Uuid::new_v4().to_string(),
        });
        Ok(())
    }

    /// Unregister all hooks for a plugin
    async fn unregister_hooks(&self, plugin_id: &str) -> Result<()> {
        let mut hooks = self.hooks.write().await;
        hooks.retain(|h| h.plugin_id != plugin_id);
        Ok(())
    }

    /// Emit an event to all registered hooks
    pub async fn emit(&self, event: PluginEvent) -> Vec<HookResult> {
        let hooks = self.hooks.read().await;
        let plugins = self.plugins.read().await;
        let mut results = Vec::new();
        
        let event_name = self.get_event_name(&event);
        
        // Sort hooks by priority
        let mut matching_hooks: Vec<_> = hooks.iter()
            .filter(|h| self.matches_pattern(&h.event_pattern, &event_name))
            .collect();
        matching_hooks.sort_by_key(|h| h.priority);
        
        for hook in matching_hooks {
            let _plugin = match plugins.get(&hook.plugin_id) {
                Some(p) if p.state == PluginState::Active => p,
                _ => continue,
            };
            
            let start = std::time::Instant::now();
            
            // In a real implementation, this would call the plugin's handler
            // For now, we just record the invocation
            let result = HookResult {
                plugin_id: hook.plugin_id.clone(),
                success: true,
                data: Some(serde_json::json!({
                    "event": event_name,
                    "handled": true
                })),
                error: None,
                execution_ms: start.elapsed().as_millis() as u64,
            };
            
            results.push(result);
        }
        
        results
    }

    fn get_event_name(&self, event: &PluginEvent) -> String {
        match event {
            PluginEvent::SessionCreated { .. } => "session.created".to_string(),
            PluginEvent::SessionUpdated { .. } => "session.updated".to_string(),
            PluginEvent::SessionDeleted { .. } => "session.deleted".to_string(),
            PluginEvent::SessionImported { .. } => "session.imported".to_string(),
            PluginEvent::SessionExported { .. } => "session.exported".to_string(),
            PluginEvent::HarvestCompleted { .. } => "harvest.completed".to_string(),
            PluginEvent::SyncCompleted { .. } => "sync.completed".to_string(),
            PluginEvent::UserAction { action, .. } => format!("user.{}", action),
            PluginEvent::AppStartup => "app.startup".to_string(),
            PluginEvent::AppShutdown => "app.shutdown".to_string(),
            PluginEvent::ConfigChanged { .. } => "config.changed".to_string(),
            PluginEvent::Custom { name, .. } => format!("custom.{}", name),
        }
    }

    fn matches_pattern(&self, pattern: &str, event_name: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        if pattern.ends_with("*") {
            let prefix = &pattern[..pattern.len() - 1];
            return event_name.starts_with(prefix);
        }
        pattern == event_name
    }

    /// Get plugin instance
    pub async fn get_plugin(&self, plugin_id: &str) -> Option<PluginInstance> {
        self.plugins.read().await.get(plugin_id).cloned()
    }

    /// List all plugins
    pub async fn list_plugins(&self) -> Vec<PluginInstance> {
        self.plugins.read().await.values().cloned().collect()
    }

    /// Get plugin configuration
    pub async fn get_config(&self, plugin_id: &str) -> Option<serde_json::Value> {
        self.plugins.read().await
            .get(plugin_id)
            .map(|p| p.config.clone())
    }

    /// Set plugin configuration
    pub async fn set_config(&self, plugin_id: &str, config: serde_json::Value) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        let plugin = plugins.get_mut(plugin_id)
            .ok_or_else(|| anyhow!("Plugin not found: {}", plugin_id))?;
        
        // Validate against schema if present
        if let Some(schema) = &plugin.manifest.config_schema {
            self.validate_config(&config, schema)?;
        }
        
        plugin.config = config;
        Ok(())
    }

    fn validate_config(&self, _config: &serde_json::Value, _schema: &serde_json::Value) -> Result<()> {
        // In a real implementation, use JSON Schema validation
        Ok(())
    }

    /// Create plugin context
    pub async fn create_context(&self, plugin_id: &str) -> Result<PluginContext> {
        let plugins = self.plugins.read().await;
        let plugin = plugins.get(plugin_id)
            .ok_or_else(|| anyhow!("Plugin not found: {}", plugin_id))?;
        
        Ok(PluginContext {
            plugin_id: plugin_id.to_string(),
            permissions: plugin.manifest.permissions.clone(),
            data_dir: plugin.path.join("data"),
            config: plugin.config.clone(),
        })
    }
}

// =============================================================================
// Plugin Registry (for discovery/installation)
// =============================================================================

/// Plugin registry entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    /// Plugin ID
    pub id: String,
    /// Latest version
    pub version: String,
    /// Plugin name
    pub name: String,
    /// Description
    pub description: String,
    /// Author
    pub author: String,
    /// Download URL
    pub download_url: String,
    /// Download count
    pub downloads: u64,
    /// Rating (0-5)
    pub rating: f32,
    /// Category
    pub category: PluginCategory,
    /// Keywords
    pub keywords: Vec<String>,
    /// Updated at
    pub updated_at: DateTime<Utc>,
}

/// Plugin registry client
pub struct PluginRegistry {
    /// Registry URL
    registry_url: String,
}

impl PluginRegistry {
    pub fn new(registry_url: String) -> Self {
        Self { registry_url }
    }

    /// Search for plugins
    pub async fn search(&self, _query: &str, _category: Option<PluginCategory>) -> Result<Vec<RegistryEntry>> {
        // In a real implementation, this would make an HTTP request
        // For now, return empty results
        Ok(Vec::new())
    }

    /// Get plugin details
    pub async fn get_plugin(&self, plugin_id: &str) -> Result<RegistryEntry> {
        Err(anyhow!("Plugin not found in registry: {}", plugin_id))
    }

    /// Download and install plugin
    pub async fn install(&self, plugin_id: &str, manager: &PluginManager) -> Result<()> {
        let _entry = self.get_plugin(plugin_id).await?;
        
        // Download plugin
        let plugin_dir = manager.plugins_dir.join(plugin_id);
        std::fs::create_dir_all(&plugin_dir)?;
        
        // In a real implementation, download and extract the plugin
        // For now, just create the directory
        
        manager.load_plugin(&plugin_dir).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_plugin_manager_init() {
        let temp_dir = tempdir().unwrap();
        let manager = PluginManager::new(temp_dir.path().to_path_buf());
        
        assert!(manager.init().await.is_ok());
    }

    #[test]
    fn test_event_name() {
        let manager = PluginManager::new(PathBuf::from("."));
        
        let event = PluginEvent::SessionCreated { session_id: "test".to_string() };
        assert_eq!(manager.get_event_name(&event), "session.created");
    }

    #[test]
    fn test_pattern_matching() {
        let manager = PluginManager::new(PathBuf::from("."));
        
        assert!(manager.matches_pattern("*", "session.created"));
        assert!(manager.matches_pattern("session.*", "session.created"));
        assert!(manager.matches_pattern("session.created", "session.created"));
        assert!(!manager.matches_pattern("session.updated", "session.created"));
    }
}
