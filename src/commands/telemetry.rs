// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Telemetry command implementations

use anyhow::Result;
use colored::*;
use std::collections::HashMap;
use std::io::{self, Write};

use crate::telemetry::{TelemetryConfig, TelemetryStore, TELEMETRY_INFO};

/// Enable telemetry (opt-in)
pub fn telemetry_opt_in() -> Result<()> {
    let mut config = TelemetryConfig::load()?;

    if config.enabled {
        println!(
            "{} Telemetry is already {}",
            "[OK]".green().bold(),
            "enabled".green()
        );
    } else {
        config.opt_in()?;
        println!(
            "{} Telemetry {} - thank you for helping improve Chasm!",
            "[OK]".green().bold(),
            "enabled".green().bold()
        );
    }

    println!();
    println!(
        "To see what data is collected, run: {}",
        "chasm telemetry info".cyan()
    );

    Ok(())
}

/// Disable telemetry (opt-out)
pub fn telemetry_opt_out() -> Result<()> {
    let mut config = TelemetryConfig::load()?;

    if !config.enabled {
        println!(
            "{} Telemetry is already {}",
            "[OK]".green().bold(),
            "disabled".yellow()
        );
    } else {
        config.opt_out()?;
        println!(
            "{} Telemetry {} - no usage data will be collected",
            "[OK]".green().bold(),
            "disabled".yellow().bold()
        );
    }

    println!();
    println!(
        "You can re-enable at any time with: {}",
        "chasm telemetry opt-in".cyan()
    );

    Ok(())
}

/// Show telemetry status and what data is collected
pub fn telemetry_info() -> Result<()> {
    let config = TelemetryConfig::load()?;

    let status = if config.enabled {
        "ENABLED (collecting anonymous data)".green().to_string()
    } else {
        "DISABLED (not collecting data)".yellow().to_string()
    };

    let info = TELEMETRY_INFO
        .replace("{installation_id}", &config.installation_id)
        .replace("{status}", &status);

    println!("{}", "[TELEMETRY INFO]".cyan().bold());
    println!("{}", info);

    // Show preference change timestamp if available
    if let Some(changed_at) = config.preference_changed_at {
        let dt = chrono::DateTime::from_timestamp(changed_at, 0)
            .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        println!("Preference last changed: {}", dt.dimmed());
    }

    println!();
    println!(
        "Use {} to change your preference",
        "chasm telemetry --help".cyan()
    );

    Ok(())
}

/// Reset telemetry ID
pub fn telemetry_reset() -> Result<()> {
    let mut config = TelemetryConfig::load()?;
    let old_id = config.installation_id.clone();

    config.reset_id()?;

    println!("{} Telemetry ID reset", "[OK]".green().bold());
    println!();
    println!("Old ID: {}", old_id.dimmed().strikethrough());
    println!("New ID: {}", config.installation_id.green());

    Ok(())
}

/// Record structured telemetry data for AI analysis
pub fn telemetry_record(
    category: &str,
    event: &str,
    data_json: Option<&str>,
    kv_pairs: &[(String, String)],
    tags: Vec<String>,
    context: Option<&str>,
    verbose: bool,
) -> Result<()> {
    let store = TelemetryStore::new()?;

    // Build data HashMap from JSON and/or key-value pairs
    let mut data: HashMap<String, serde_json::Value> = if let Some(json_str) = data_json {
        serde_json::from_str(json_str).map_err(|e| {
            anyhow::anyhow!(
                "Invalid JSON data: {}. Use valid JSON or --kv key=value pairs",
                e
            )
        })?
    } else {
        HashMap::new()
    };

    // Add key-value pairs (these override JSON values if keys conflict)
    for (key, value) in kv_pairs {
        // Try to parse value as JSON, fallback to string
        let json_value =
            serde_json::from_str(value).unwrap_or(serde_json::Value::String(value.clone()));
        data.insert(key.clone(), json_value);
    }

    let record = store.record(category, event, data, tags, context.map(|s| s.to_string()))?;

    println!(
        "{} Recorded telemetry event: {}",
        "[OK]".green().bold(),
        format!("{}:{}", record.category, record.event).cyan()
    );

    if verbose {
        println!();
        println!("Record ID:   {}", record.id.dimmed());
        println!("Timestamp:   {}", record.timestamp_iso);
        println!("Category:    {}", record.category.cyan());
        println!("Event:       {}", record.event);
        if !record.tags.is_empty() {
            println!("Tags:        {}", record.tags.join(", ").yellow());
        }
        if let Some(ctx) = &record.context {
            println!("Context:     {}", ctx);
        }
        if !record.data.is_empty() {
            println!("Data:");
            let pretty = serde_json::to_string_pretty(&record.data).unwrap_or_default();
            for line in pretty.lines() {
                println!("  {}", line);
            }
        }
    }

    Ok(())
}

/// Show recorded telemetry data
pub fn telemetry_show(
    category: Option<&str>,
    event: Option<&str>,
    tag: Option<&str>,
    limit: usize,
    format: &str,
    after: Option<&str>,
    before: Option<&str>,
) -> Result<()> {
    let store = TelemetryStore::new()?;

    // Parse date filters
    let after_ts = after.and_then(|d| parse_date_to_timestamp(d));
    let before_ts = before.and_then(|d| parse_date_to_timestamp(d));

    let records = store.read_records(category, event, tag, after_ts, before_ts, Some(limit))?;

    if records.is_empty() {
        println!("{} No telemetry records found", "[INFO]".cyan());
        return Ok(());
    }

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&records)?;
            println!("{}", json);
        }
        "jsonl" => {
            for record in &records {
                println!("{}", serde_json::to_string(record)?);
            }
        }
        _ => {
            // Table format (default)
            println!(
                "{} Showing {} telemetry records",
                "[TELEMETRY]".cyan().bold(),
                records.len().to_string().green()
            );
            println!();

            for record in &records {
                let time_short = &record.timestamp_iso[..19];
                let tags_str = if record.tags.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", record.tags.join(", ").yellow())
                };

                println!(
                    "{} {} {}{}",
                    time_short.dimmed(),
                    record.category.cyan(),
                    record.event.white().bold(),
                    tags_str
                );

                if !record.data.is_empty() {
                    let data_str = serde_json::to_string(&record.data).unwrap_or_default();
                    // Truncate long data
                    let display = if data_str.len() > 80 {
                        format!("{}...", &data_str[..77])
                    } else {
                        data_str
                    };
                    println!("           {}", display.dimmed());
                }
            }

            let total = store.count_records()?;
            if total > limit {
                println!();
                println!(
                    "Showing {} of {} total records. Use {} to see more.",
                    limit,
                    total,
                    "-n <limit>".cyan()
                );
            }
        }
    }

    Ok(())
}

/// Export telemetry records
pub fn telemetry_export(
    output: &str,
    format: &str,
    category: Option<&str>,
    with_metadata: bool,
) -> Result<()> {
    let store = TelemetryStore::new()?;

    let count = store.export_records(output, format, category, with_metadata)?;

    if count == 0 {
        println!("{} No records to export", "[INFO]".yellow());
    } else {
        println!(
            "{} Exported {} records to {}",
            "[OK]".green().bold(),
            count.to_string().cyan(),
            output.green()
        );

        if with_metadata {
            println!("   Installation ID included in export");
        }
    }

    Ok(())
}

/// Clear telemetry records
pub fn telemetry_clear(force: bool, older_than: Option<u32>) -> Result<()> {
    let store = TelemetryStore::new()?;
    let count = store.count_records()?;

    if count == 0 {
        println!("{} No telemetry records to clear", "[INFO]".cyan());
        return Ok(());
    }

    let message = if let Some(days) = older_than {
        format!(
            "Clear {} telemetry records older than {} days?",
            count, days
        )
    } else {
        format!("Clear all {} telemetry records?", count)
    };

    if !force {
        print!("{} [y/N] ", message.yellow());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled");
            return Ok(());
        }
    }

    let removed = store.clear_records(older_than)?;

    println!(
        "{} Cleared {} telemetry records",
        "[OK]".green().bold(),
        removed.to_string().cyan()
    );

    Ok(())
}

/// Configure remote telemetry endpoint
pub fn telemetry_config(
    endpoint: Option<&str>,
    api_key: Option<&str>,
    enable_remote: bool,
    disable_remote: bool,
) -> Result<()> {
    let mut config = TelemetryConfig::load()?;
    let mut changed = false;

    if let Some(ep) = endpoint {
        config.set_remote_endpoint(Some(ep.to_string()))?;
        println!(
            "{} Remote endpoint set to: {}",
            "[OK]".green().bold(),
            ep.cyan()
        );
        changed = true;
    }

    if let Some(key) = api_key {
        config.set_remote_api_key(Some(key.to_string()))?;
        // Don't print the actual key for security
        println!(
            "{} API key configured ({})",
            "[OK]".green().bold(),
            format!("{}...", &key[..key.len().min(8)]).dimmed()
        );
        changed = true;
    }

    if enable_remote {
        config.set_remote_enabled(true)?;
        println!("{} Remote telemetry enabled", "[OK]".green().bold());
        changed = true;
    }

    if disable_remote {
        config.set_remote_enabled(false)?;
        println!("{} Remote telemetry disabled", "[OK]".green().bold());
        changed = true;
    }

    // Reload config to show current state
    let config = TelemetryConfig::load()?;

    if !changed {
        // Just show current config
        println!("{}", "[REMOTE TELEMETRY CONFIG]".cyan().bold());
        println!();
        println!(
            "Endpoint:    {}",
            config
                .remote_endpoint
                .as_deref()
                .unwrap_or("(not configured)")
                .cyan()
        );
        println!(
            "API Key:     {}",
            if config.remote_api_key.is_some() {
                "(configured)".green().to_string()
            } else {
                "(not configured)".yellow().to_string()
            }
        );
        println!(
            "Remote Send: {}",
            if config.remote_enabled {
                "ENABLED".green().bold().to_string()
            } else {
                "DISABLED".yellow().to_string()
            }
        );

        if config.is_remote_enabled() {
            println!();
            println!(
                "{} Ready to sync. Use {}",
                "[✓]".green(),
                "chasm telemetry sync".cyan()
            );
        } else if config.remote_endpoint.is_some() && config.remote_api_key.is_some() {
            println!();
            println!(
                "{} Configured but disabled. Use {}",
                "[!]".yellow(),
                "--enable-remote".cyan()
            );
        } else {
            println!();
            println!("To configure:");
            println!(
                "  {} {}",
                "chasm telemetry config".cyan(),
                "--endpoint <URL> --api-key <KEY> --enable-remote"
            );
        }
    }

    Ok(())
}

/// Sync telemetry records to remote server
pub fn telemetry_sync(limit: Option<usize>, clear_after: bool) -> Result<()> {
    let store = TelemetryStore::new()?;

    let count = store.count_records()?;
    if count == 0 {
        println!("{} No telemetry records to sync", "[INFO]".cyan());
        return Ok(());
    }

    println!(
        "{} Syncing {} telemetry records to remote server...",
        "[SYNC]".cyan().bold(),
        limit.unwrap_or(count).min(count).to_string().green()
    );

    let result = store.sync_to_remote(limit)?;

    if result.success {
        println!(
            "{} Successfully sent {} records",
            "[OK]".green().bold(),
            result.records_sent.to_string().cyan()
        );

        if clear_after && result.records_sent > 0 {
            let cleared = store.clear_records(None)?;
            println!("   Cleared {} local records", cleared.to_string().dimmed());
        }
    } else {
        println!(
            "{} Sync failed: {}",
            "[ERROR]".red().bold(),
            result.error.unwrap_or_else(|| "Unknown error".to_string())
        );
    }

    Ok(())
}

/// Test connection to remote telemetry server
pub fn telemetry_test() -> Result<()> {
    let config = TelemetryConfig::load()?;

    if config.remote_endpoint.is_none() {
        println!("{} Remote endpoint not configured", "[ERROR]".red().bold());
        println!(
            "   Use: {}",
            "chasm telemetry config --endpoint <URL>".cyan()
        );
        return Ok(());
    }

    let endpoint = config.remote_endpoint.as_ref().unwrap();
    println!(
        "{} Testing connection to {}",
        "[TEST]".cyan().bold(),
        endpoint
    );

    // Try health endpoint
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {}", e))?;

    let health_url = format!("{}/health", endpoint.trim_end_matches('/'));
    let response = client.get(&health_url).send();

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let body: serde_json::Value = resp.json().unwrap_or_default();
                println!("{} Connection successful!", "[OK]".green().bold());
                println!();
                println!(
                    "Server status: {}",
                    body.get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .green()
                );
                if let Some(env) = body.get("environment").and_then(|v| v.as_str()) {
                    println!("Environment:   {}", env);
                }

                // Test auth if API key is configured
                if config.remote_api_key.is_some() {
                    println!();
                    println!("{} API key configured", "[✓]".green());
                } else {
                    println!();
                    println!(
                        "{} API key not configured (required for syncing)",
                        "[!]".yellow()
                    );
                }
            } else {
                println!(
                    "{} Server returned: HTTP {}",
                    "[WARN]".yellow().bold(),
                    resp.status()
                );
            }
        }
        Err(e) => {
            println!("{} Connection failed: {}", "[ERROR]".red().bold(), e);
            println!();
            println!("Please check:");
            println!("  • The endpoint URL is correct");
            println!("  • The server is running");
            println!("  • Your network connection");
        }
    }

    Ok(())
}

/// Parse a date string (YYYY-MM-DD) to a Unix timestamp
fn parse_date_to_timestamp(date_str: &str) -> Option<i64> {
    chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .ok()
        .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp())
}
