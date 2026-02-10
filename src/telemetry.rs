// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Telemetry module for anonymous usage data collection
//!
//! This module provides opt-in (by default) anonymous usage telemetry to help
//! improve Chasm. No personal data is collected - only aggregate usage statistics.

use crate::error::{CsmError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use uuid::Uuid;

/// Telemetry configuration stored on disk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Whether telemetry is enabled (opt-in by default)
    pub enabled: bool,

    /// Anonymous identifier for this installation
    pub installation_id: String,

    /// When the config was first created
    pub created_at: i64,

    /// When the user last changed their preference
    pub preference_changed_at: Option<i64>,

    /// Version of the config format
    pub version: u32,

    /// Remote telemetry endpoint URL (optional)
    #[serde(default)]
    pub remote_endpoint: Option<String>,

    /// API key for remote endpoint (optional)
    #[serde(default)]
    pub remote_api_key: Option<String>,

    /// Whether to send telemetry to remote endpoint
    #[serde(default)]
    pub remote_enabled: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: true, // Opt-in by default as requested
            installation_id: Uuid::new_v4().to_string(),
            created_at: chrono::Utc::now().timestamp(),
            preference_changed_at: None,
            version: 1,
            remote_endpoint: None,
            remote_api_key: None,
            remote_enabled: false,
        }
    }
}

impl TelemetryConfig {
    /// Get the path to the telemetry config file
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = if cfg!(target_os = "windows") {
            dirs::config_dir().map(|p| p.join("chasm"))
        } else if cfg!(target_os = "macos") {
            dirs::home_dir().map(|p| p.join(".config/chasm"))
        } else {
            dirs::home_dir().map(|p| p.join(".config/chasm"))
        };

        config_dir
            .map(|p| p.join("telemetry.json"))
            .ok_or(CsmError::StorageNotFound)
    }

    /// Load telemetry config from disk, creating default if not exists
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            serde_json::from_str(&content)
                .map_err(|e| CsmError::InvalidSessionFormat(e.to_string()))
        } else {
            // Create default config (opt-in by default)
            let config = Self::default();
            config.save()?;
            Ok(config)
        }
    }

    /// Save telemetry config to disk
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| CsmError::InvalidSessionFormat(e.to_string()))?;
        fs::write(&config_path, content)?;

        Ok(())
    }

    /// Enable telemetry
    pub fn opt_in(&mut self) -> Result<()> {
        self.enabled = true;
        self.preference_changed_at = Some(chrono::Utc::now().timestamp());
        self.save()
    }

    /// Disable telemetry
    pub fn opt_out(&mut self) -> Result<()> {
        self.enabled = false;
        self.preference_changed_at = Some(chrono::Utc::now().timestamp());
        self.save()
    }

    /// Reset installation ID (generates new anonymous identifier)
    pub fn reset_id(&mut self) -> Result<()> {
        self.installation_id = Uuid::new_v4().to_string();
        self.preference_changed_at = Some(chrono::Utc::now().timestamp());
        self.save()
    }

    /// Check if telemetry is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Configure remote endpoint
    pub fn set_remote_endpoint(&mut self, endpoint: Option<String>) -> Result<()> {
        self.remote_endpoint = endpoint;
        self.preference_changed_at = Some(chrono::Utc::now().timestamp());
        self.save()
    }

    /// Configure remote API key
    pub fn set_remote_api_key(&mut self, api_key: Option<String>) -> Result<()> {
        self.remote_api_key = api_key;
        self.preference_changed_at = Some(chrono::Utc::now().timestamp());
        self.save()
    }

    /// Enable/disable remote sending
    pub fn set_remote_enabled(&mut self, enabled: bool) -> Result<()> {
        self.remote_enabled = enabled;
        self.preference_changed_at = Some(chrono::Utc::now().timestamp());
        self.save()
    }

    /// Check if remote telemetry is configured and enabled
    pub fn is_remote_enabled(&self) -> bool {
        self.remote_enabled && self.remote_endpoint.is_some() && self.remote_api_key.is_some()
    }
}

/// Types of telemetry events we track
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TelemetryEvent {
    /// CLI command invoked
    CommandInvoked {
        command: String,
        subcommand: Option<String>,
        duration_ms: Option<u64>,
        success: bool,
    },

    /// Session harvested from a provider
    SessionHarvested {
        provider: String,
        session_count: u32,
    },

    /// Sessions merged
    SessionsMerged { session_count: u32 },

    /// API server started
    ApiServerStarted { port: u16 },

    /// Provider detected
    ProviderDetected { provider: String },

    /// Error occurred (no PII, just error type)
    ErrorOccurred { error_type: String },
}

/// Telemetry collector that batches and sends events
#[derive(Debug)]
pub struct TelemetryCollector {
    config: TelemetryConfig,
    events: Vec<TelemetryEvent>,
}

impl TelemetryCollector {
    /// Create a new telemetry collector
    pub fn new() -> Result<Self> {
        let config = TelemetryConfig::load()?;
        Ok(Self {
            config,
            events: Vec::new(),
        })
    }

    /// Check if telemetry is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.is_enabled()
    }

    /// Track a telemetry event
    pub fn track(&mut self, event: TelemetryEvent) {
        if self.is_enabled() {
            self.events.push(event);
        }
    }

    /// Track a CLI command invocation
    pub fn track_command(&mut self, command: &str, subcommand: Option<&str>, success: bool) {
        self.track(TelemetryEvent::CommandInvoked {
            command: command.to_string(),
            subcommand: subcommand.map(|s| s.to_string()),
            duration_ms: None,
            success,
        });
    }

    /// Get the installation ID
    pub fn installation_id(&self) -> &str {
        &self.config.installation_id
    }

    /// Flush events (in future: send to telemetry endpoint)
    /// Currently just clears the buffer - actual sending will be implemented later
    pub fn flush(&mut self) -> Result<()> {
        if !self.is_enabled() || self.events.is_empty() {
            return Ok(());
        }

        // TODO: In future versions, send events to telemetry endpoint
        // For now, we just clear the buffer
        // The endpoint and sending logic will be added when the backend is ready

        self.events.clear();
        Ok(())
    }
}

impl Drop for TelemetryCollector {
    fn drop(&mut self) {
        // Try to flush remaining events on drop
        let _ = self.flush();
    }
}

/// What data is collected (for user information)
pub const TELEMETRY_INFO: &str = r#"
Chasm collects anonymous usage data to help improve the product.

WHAT WE COLLECT:
  • Commands used (e.g., 'harvest', 'merge', 'export')
  • Provider types detected (e.g., 'copilot', 'cursor', 'ollama')
  • Session counts (numbers only, no content)
  • Error types (no personal details or file paths)
  • Anonymous installation ID (randomly generated UUID)

WHAT WE DO NOT COLLECT:
  • Chat messages or content
  • File paths or project names
  • Personal information
  • API keys or credentials
  • IP addresses (beyond what's needed for HTTPS)

Your installation ID: {installation_id}
Status: {status}

Manage your preference:
  chasm telemetry opt-in   - Enable data collection (default)
  chasm telemetry opt-out  - Disable data collection
  chasm telemetry reset    - Generate new anonymous ID
"#;

// =============================================================================
// STRUCTURED DATA RECORDING FOR AI ANALYSIS
// =============================================================================

/// A structured telemetry record for AI analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryRecord {
    /// Unique record ID
    pub id: String,

    /// Installation ID (anonymous)
    pub installation_id: String,

    /// Event category (e.g., 'workflow', 'error', 'performance', 'usage', 'custom')
    pub category: String,

    /// Event name or type
    pub event: String,

    /// Structured data payload
    pub data: HashMap<String, serde_json::Value>,

    /// Tags for filtering
    pub tags: Vec<String>,

    /// Optional context/session ID
    pub context: Option<String>,

    /// Unix timestamp when recorded
    pub timestamp: i64,

    /// Human-readable timestamp
    pub timestamp_iso: String,
}

impl TelemetryRecord {
    /// Create a new telemetry record
    pub fn new(
        installation_id: &str,
        category: &str,
        event: &str,
        data: HashMap<String, serde_json::Value>,
        tags: Vec<String>,
        context: Option<String>,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            installation_id: installation_id.to_string(),
            category: category.to_string(),
            event: event.to_string(),
            data,
            tags,
            context,
            timestamp: now.timestamp(),
            timestamp_iso: now.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
        }
    }
}

/// Storage for telemetry records (JSONL file for easy streaming/appending)
pub struct TelemetryStore {
    config: TelemetryConfig,
}

impl TelemetryStore {
    /// Create a new telemetry store
    pub fn new() -> Result<Self> {
        let config = TelemetryConfig::load()?;
        Ok(Self { config })
    }

    /// Get path to the telemetry records file
    pub fn records_path() -> Result<PathBuf> {
        let config_dir = if cfg!(target_os = "windows") {
            dirs::config_dir().map(|p| p.join("chasm"))
        } else {
            dirs::home_dir().map(|p| p.join(".config/chasm"))
        };

        config_dir
            .map(|p| p.join("telemetry_records.jsonl"))
            .ok_or(CsmError::StorageNotFound)
    }

    /// Record a new telemetry event
    pub fn record(
        &self,
        category: &str,
        event: &str,
        data: HashMap<String, serde_json::Value>,
        tags: Vec<String>,
        context: Option<String>,
    ) -> Result<TelemetryRecord> {
        let record = TelemetryRecord::new(
            &self.config.installation_id,
            category,
            event,
            data,
            tags,
            context,
        );

        // Append to JSONL file
        let path = Self::records_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = OpenOptions::new().create(true).append(true).open(&path)?;

        let line = serde_json::to_string(&record)
            .map_err(|e| CsmError::InvalidSessionFormat(e.to_string()))?;
        writeln!(file, "{}", line)?;

        Ok(record)
    }

    /// Read all records, optionally filtered
    pub fn read_records(
        &self,
        category: Option<&str>,
        event: Option<&str>,
        tag: Option<&str>,
        after: Option<i64>,
        before: Option<i64>,
        limit: Option<usize>,
    ) -> Result<Vec<TelemetryRecord>> {
        let path = Self::records_path()?;
        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        let mut records: Vec<TelemetryRecord> = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            let record: TelemetryRecord = serde_json::from_str(&line)
                .map_err(|e| CsmError::InvalidSessionFormat(e.to_string()))?;

            // Apply filters
            if let Some(cat) = category {
                if record.category != cat {
                    continue;
                }
            }
            if let Some(evt) = event {
                if record.event != evt {
                    continue;
                }
            }
            if let Some(t) = tag {
                if !record.tags.contains(&t.to_string()) {
                    continue;
                }
            }
            if let Some(after_ts) = after {
                if record.timestamp < after_ts {
                    continue;
                }
            }
            if let Some(before_ts) = before {
                if record.timestamp > before_ts {
                    continue;
                }
            }

            records.push(record);
        }

        // Sort by timestamp descending (newest first)
        records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Apply limit
        if let Some(lim) = limit {
            records.truncate(lim);
        }

        Ok(records)
    }

    /// Get record count
    pub fn count_records(&self) -> Result<usize> {
        let path = Self::records_path()?;
        if !path.exists() {
            return Ok(0);
        }

        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        Ok(reader.lines().filter(|l| l.is_ok()).count())
    }

    /// Clear records (optionally older than N days)
    pub fn clear_records(&self, older_than_days: Option<u32>) -> Result<usize> {
        let path = Self::records_path()?;
        if !path.exists() {
            return Ok(0);
        }

        if older_than_days.is_none() {
            // Delete entire file
            let count = self.count_records()?;
            fs::remove_file(&path)?;
            return Ok(count);
        }

        // Filter out old records
        let cutoff =
            chrono::Utc::now().timestamp() - (older_than_days.unwrap() as i64 * 24 * 60 * 60);

        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        let mut kept_records: Vec<String> = Vec::new();
        let mut removed_count = 0;

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            let record: TelemetryRecord = serde_json::from_str(&line)
                .map_err(|e| CsmError::InvalidSessionFormat(e.to_string()))?;

            if record.timestamp >= cutoff {
                kept_records.push(line);
            } else {
                removed_count += 1;
            }
        }

        // Rewrite file with kept records
        let mut file = File::create(&path)?;
        for line in kept_records {
            writeln!(file, "{}", line)?;
        }

        Ok(removed_count)
    }

    /// Export records to a file
    pub fn export_records(
        &self,
        output_path: &str,
        format: &str,
        category: Option<&str>,
        with_metadata: bool,
    ) -> Result<usize> {
        let records = self.read_records(category, None, None, None, None, None)?;

        if records.is_empty() {
            return Ok(0);
        }

        let mut file = File::create(output_path)?;

        match format {
            "json" => {
                if with_metadata {
                    let export = serde_json::json!({
                        "installation_id": self.config.installation_id,
                        "exported_at": chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
                        "record_count": records.len(),
                        "records": records,
                    });
                    let content = serde_json::to_string_pretty(&export)
                        .map_err(|e| CsmError::InvalidSessionFormat(e.to_string()))?;
                    write!(file, "{}", content)?;
                } else {
                    let content = serde_json::to_string_pretty(&records)
                        .map_err(|e| CsmError::InvalidSessionFormat(e.to_string()))?;
                    write!(file, "{}", content)?;
                }
            }
            "jsonl" => {
                if with_metadata {
                    let meta = serde_json::json!({
                        "_type": "metadata",
                        "installation_id": self.config.installation_id,
                        "exported_at": chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
                        "record_count": records.len(),
                    });
                    writeln!(
                        file,
                        "{}",
                        serde_json::to_string(&meta)
                            .map_err(|e| CsmError::InvalidSessionFormat(e.to_string()))?
                    )?;
                }
                for record in &records {
                    let line = serde_json::to_string(record)
                        .map_err(|e| CsmError::InvalidSessionFormat(e.to_string()))?;
                    writeln!(file, "{}", line)?;
                }
            }
            "csv" => {
                // Write CSV header
                writeln!(
                    file,
                    "id,timestamp,timestamp_iso,category,event,tags,context,data"
                )?;
                for record in &records {
                    let tags = record.tags.join(";");
                    let context = record.context.clone().unwrap_or_default();
                    let data = serde_json::to_string(&record.data)
                        .map_err(|e| CsmError::InvalidSessionFormat(e.to_string()))?;
                    // Escape CSV fields
                    let data_escaped = data.replace('"', "\"\"");
                    writeln!(
                        file,
                        "{},{},\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"",
                        record.id,
                        record.timestamp,
                        record.timestamp_iso,
                        record.category,
                        record.event,
                        tags,
                        context,
                        data_escaped
                    )?;
                }
            }
            _ => {
                return Err(CsmError::InvalidSessionFormat(format!(
                    "Unknown export format: {}",
                    format
                ))
                .into());
            }
        }

        Ok(records.len())
    }

    /// Get installation ID
    pub fn installation_id(&self) -> &str {
        &self.config.installation_id
    }

    /// Sync records to remote endpoint
    pub fn sync_to_remote(&self, limit: Option<usize>) -> Result<SyncResult> {
        if !self.config.is_remote_enabled() {
            return Err(CsmError::InvalidSessionFormat(
                "Remote telemetry not configured. Use 'chasm telemetry config' to set endpoint and API key".to_string()
            ).into());
        }

        let endpoint = self.config.remote_endpoint.as_ref().unwrap();
        let api_key = self.config.remote_api_key.as_ref().unwrap();

        // Read records to sync
        let records = self.read_records(None, None, None, None, None, limit)?;

        if records.is_empty() {
            return Ok(SyncResult {
                records_sent: 0,
                success: true,
                error: None,
            });
        }

        // Build the request payload
        let payload = serde_json::json!({
            "installation_id": self.config.installation_id,
            "records": records,
        });

        // Send to remote endpoint
        let client = reqwest::blocking::Client::new();
        let response = client
            .post(format!("{}/ingest", endpoint.trim_end_matches('/')))
            .header("Content-Type", "application/json")
            .header("X-Api-Key", api_key)
            .json(&payload)
            .send();

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    Ok(SyncResult {
                        records_sent: records.len(),
                        success: true,
                        error: None,
                    })
                } else {
                    let status = resp.status();
                    let error_text = resp.text().unwrap_or_else(|_| "Unknown error".to_string());
                    Ok(SyncResult {
                        records_sent: 0,
                        success: false,
                        error: Some(format!("HTTP {}: {}", status, error_text)),
                    })
                }
            }
            Err(e) => Ok(SyncResult {
                records_sent: 0,
                success: false,
                error: Some(format!("Request failed: {}", e)),
            }),
        }
    }

    /// Get the config
    pub fn config(&self) -> &TelemetryConfig {
        &self.config
    }
}

/// Result of a sync operation
#[derive(Debug)]
pub struct SyncResult {
    pub records_sent: usize,
    pub success: bool,
    pub error: Option<String>,
}
