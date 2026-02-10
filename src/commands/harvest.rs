// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Harvest commands for collecting chat sessions from multiple providers
//!
//! The harvester scans for local and remote LLM providers, downloads all chat
//! sessions into a single SQLite database, and allows users to version-track
//! the database with git.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use colored::*;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::browser::{get_installed_browsers, scan_browser_auth, BrowserType};
use crate::database::{ChatDatabase, ShareLinkParser};
use crate::models::ChatSession;
use crate::providers::{ProviderRegistry, ProviderType};
use crate::storage::parse_session_json;
use crate::workspace::{discover_workspaces, get_chat_sessions_from_workspace};

/// Check if a string is an empty code block marker (just ``` with no content)
fn is_empty_code_block(s: &str) -> bool {
    // Match patterns like "```", "```\n", "```language", "```\n```", "```\n\n```"
    let s = s.trim();
    if s == "```" {
        return true;
    }
    // Check for code block with just a language identifier and no content
    if s.starts_with("```") && !s.contains('\n') {
        return true;
    }
    // Check for empty code block with opening and closing (possibly with whitespace-only lines)
    let lines: Vec<&str> = s.lines().collect();
    if lines.len() >= 2 && lines[0].starts_with("```") && lines.last() == Some(&"```") {
        // Check if all lines between opening and closing are empty or whitespace
        let content_lines = &lines[1..lines.len() - 1];
        if content_lines.iter().all(|line| line.trim().is_empty()) {
            return true;
        }
    }
    false
}

/// Type alias for harvested session query result (id, provider, title, msg_count, created, last_msg, workspace)
type HarvestQueryResult = (String, String, String, i64, i64, i64, Option<String>);

/// Web-based LLM provider endpoint configuration
#[derive(Debug, Clone)]
struct WebProviderEndpoint {
    name: &'static str,
    url: &'static str,
    description: &'static str,
}

/// List of known web-based LLM provider endpoints to probe
const WEB_PROVIDERS: &[WebProviderEndpoint] = &[
    WebProviderEndpoint {
        name: "ChatGPT",
        url: "https://chat.openai.com",
        description: "OpenAI ChatGPT Web Interface",
    },
    WebProviderEndpoint {
        name: "Claude",
        url: "https://claude.ai",
        description: "Anthropic Claude Web Interface",
    },
    WebProviderEndpoint {
        name: "Gemini",
        url: "https://gemini.google.com",
        description: "Google Gemini Web Interface",
    },
    WebProviderEndpoint {
        name: "Perplexity",
        url: "https://www.perplexity.ai",
        description: "Perplexity AI Search Interface",
    },
    WebProviderEndpoint {
        name: "DeepSeek",
        url: "https://chat.deepseek.com",
        description: "DeepSeek Chat Interface",
    },
    WebProviderEndpoint {
        name: "Poe",
        url: "https://poe.com",
        description: "Quora Poe Multi-model Chat",
    },
    WebProviderEndpoint {
        name: "You.com",
        url: "https://you.com/chat",
        description: "You.com AI Chat",
    },
    WebProviderEndpoint {
        name: "HuggingChat",
        url: "https://huggingface.co/chat",
        description: "HuggingFace Chat Interface",
    },
    WebProviderEndpoint {
        name: "Copilot",
        url: "https://copilot.microsoft.com",
        description: "Microsoft Copilot Web Interface",
    },
    WebProviderEndpoint {
        name: "Mistral",
        url: "https://chat.mistral.ai",
        description: "Mistral AI Le Chat Interface",
    },
    WebProviderEndpoint {
        name: "Cohere",
        url: "https://coral.cohere.com",
        description: "Cohere Coral Chat Interface",
    },
    WebProviderEndpoint {
        name: "Groq",
        url: "https://groq.com",
        description: "Groq Fast Inference Interface",
    },
    WebProviderEndpoint {
        name: "Phind",
        url: "https://www.phind.com",
        description: "Phind AI Code Search",
    },
    WebProviderEndpoint {
        name: "Character.AI",
        url: "https://character.ai",
        description: "Character.AI Chat Interface",
    },
    WebProviderEndpoint {
        name: "Pi",
        url: "https://pi.ai",
        description: "Inflection Pi Personal AI",
    },
];

/// Scan for reachable web-based LLM providers (parallel)
fn scan_web_providers(timeout_secs: u64) -> Vec<String> {
    use std::sync::{Arc, Mutex};
    use std::thread;

    let reachable = Arc::new(Mutex::new(Vec::new()));
    let results = Arc::new(Mutex::new(Vec::new()));

    // Create threads for parallel scanning
    let handles: Vec<_> = WEB_PROVIDERS
        .iter()
        .map(|provider| {
            let reachable = Arc::clone(&reachable);
            let results = Arc::clone(&results);
            let timeout = timeout_secs;
            let name = provider.name;
            let url = provider.url;
            let desc = provider.description;

            thread::spawn(move || {
                let client = match reqwest::blocking::Client::builder()
                    .timeout(Duration::from_secs(timeout))
                    .connect_timeout(Duration::from_secs(timeout.min(3))) // Short connect timeout
                    .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                    .build()
                {
                    Ok(c) => c,
                    Err(_) => return,
                };

                let result: (&str, bool, String, &str) = match client.head(url).send() {
                    Ok(response) => {
                        let status = response.status();
                        if status.is_success() || status.is_redirection() {
                            reachable.lock().unwrap().push(name.to_string());
                            (name, true, desc.to_string(), url)
                        } else {
                            (name, false, format!("HTTP {}", status.as_u16()), url)
                        }
                    }
                    Err(e) => {
                        let reason = if e.is_timeout() {
                            "timeout".to_string()
                        } else if e.is_connect() {
                            "connection failed".to_string()
                        } else {
                            "unreachable".to_string()
                        };
                        (name, false, reason, url)
                    }
                };

                results.lock().unwrap().push((
                    result.0.to_string(),
                    result.1,
                    result.2,
                    result.3.to_string(),
                ));
            })
        })
        .collect();

    // Wait for all threads to complete
    for handle in handles {
        let _ = handle.join();
    }

    // Print results in order
    let results = results.lock().unwrap();
    let mut sorted_results: Vec<_> = results.iter().collect();
    sorted_results.sort_by_key(|(name, _, _, _)| name.as_str());

    for (name, success, info, url) in sorted_results {
        if *success {
            println!(
                "   {} {}: {} ({})",
                "[+]".green(),
                name.bold(),
                "reachable".green(),
                info.dimmed()
            );
            println!("      {} {}", "`".dimmed(), url.dimmed());
        } else {
            println!(
                "   {} {}: {} ({})",
                "[-]".dimmed(),
                name,
                "blocked or unavailable".dimmed(),
                info.dimmed()
            );
        }
    }

    let result = reachable.lock().unwrap().clone();
    result
}

/// Scan browser cookies for authenticated web LLM providers
fn scan_browser_authentication(
    verbose: bool,
) -> (std::collections::HashMap<String, Vec<BrowserType>>, usize) {
    use crate::browser::scan_browser_auth_verbose;
    use std::collections::HashMap;

    let installed = get_installed_browsers();
    if installed.is_empty() {
        println!("   {} No supported browsers found", "[-]".dimmed());
        return (HashMap::new(), 0);
    }

    println!(
        "   {} Checking {} browser(s): {}",
        "[*]".blue(),
        installed.len(),
        installed
            .iter()
            .map(|b| b.name())
            .collect::<Vec<_>>()
            .join(", ")
    );

    let results = if verbose {
        scan_browser_auth_verbose()
    } else {
        scan_browser_auth()
    };

    // Group results by provider
    let mut authenticated: HashMap<String, Vec<BrowserType>> = HashMap::new();

    for result in results {
        if result.authenticated {
            authenticated
                .entry(result.provider.clone())
                .or_default()
                .push(result.browser);
        }
    }

    let count = authenticated.len();

    if authenticated.is_empty() {
        println!(
            "   {} No authenticated web LLM providers found",
            "[-]".dimmed()
        );
        println!(
            "      {} Log into ChatGPT, Claude, etc. in your browser to enable harvesting",
            "`".dimmed()
        );
    } else {
        for (provider, browsers) in &authenticated {
            let browser_names: Vec<_> = browsers.iter().map(|b| b.name()).collect();
            println!(
                "   {} {}: {} in {}",
                "[+]".green(),
                provider.bold(),
                "authenticated".green(),
                browser_names.join(", ")
            );
        }
    }

    (authenticated, count)
}

/// Configuration for the harvest database
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarvestConfig {
    /// Path to the harvest database
    pub db_path: PathBuf,
    /// Auto-commit changes to git
    pub auto_commit: bool,
    /// Providers to include (empty = all)
    pub include_providers: Vec<String>,
    /// Providers to exclude
    pub exclude_providers: Vec<String>,
    /// Include VS Code workspaces
    pub include_workspaces: bool,
    /// Last harvest timestamp
    pub last_harvest: Option<i64>,
}

#[allow(dead_code)]
impl Default for HarvestConfig {
    fn default() -> Self {
        Self {
            db_path: PathBuf::from("chat_sessions.db"),
            auto_commit: false,
            include_providers: Vec::new(),
            exclude_providers: Vec::new(),
            include_workspaces: true,
            last_harvest: None,
        }
    }
}

/// Statistics from a harvest operation
#[derive(Debug, Default)]
pub struct HarvestStats {
    pub providers_scanned: usize,
    pub workspaces_scanned: usize,
    pub sessions_found: usize,
    pub sessions_added: usize,
    pub sessions_updated: usize,
    pub sessions_skipped: usize,
    pub errors: Vec<String>,
}

/// A harvested session record
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarvestedSession {
    pub id: String,
    pub provider: String,
    pub provider_type: String,
    pub workspace_id: Option<String>,
    pub workspace_name: Option<String>,
    pub title: String,
    pub message_count: usize,
    pub created_at: i64,
    pub updated_at: i64,
    pub harvested_at: i64,
    pub session_json: String,
}

/// Initialize a harvest database at the specified path
pub fn harvest_init(path: Option<&str>, git_init: bool) -> Result<()> {
    let db_path = get_db_path(path)?;
    let db_dir = db_path.parent().unwrap_or(Path::new("."));

    println!("\n{} Initializing Harvest Database", "[H]".magenta().bold());
    println!("{}", "=".repeat(60));

    // Create directory if needed
    if !db_dir.exists() {
        fs::create_dir_all(db_dir)?;
        println!("{} Created directory: {}", "[+]".green(), db_dir.display());
    }

    // Check if database already exists
    if db_path.exists() {
        println!(
            "{} Database already exists: {}",
            "[!]".yellow(),
            db_path.display()
        );
        println!("   Use 'csm harvest run' to update it");
        return Ok(());
    }

    // Create the database
    create_harvest_database(&db_path)?;
    println!("{} Created database: {}", "[+]".green(), db_path.display());

    // Initialize git if requested
    if git_init {
        init_git_tracking(&db_path)?;
    }

    println!("\n{} Harvest database initialized!", "[+]".green().bold());
    println!("\nNext steps:");
    println!("  1. Run 'csm harvest scan' to see available providers");
    println!("  2. Run 'csm harvest run' to collect sessions");
    if !git_init {
        println!("  3. Run 'csm harvest git init' to enable version tracking");
    }

    Ok(())
}

/// Scan for available providers and workspaces
pub fn harvest_scan(
    show_sessions: bool,
    scan_web: bool,
    timeout_secs: u64,
    verbose: bool,
) -> Result<()> {
    println!("\n{} Scanning for Providers", "[H]".magenta().bold());
    println!("{}", "=".repeat(60));

    let registry = ProviderRegistry::new();
    let mut total_sessions = 0;
    let mut available_providers = Vec::new();

    // Scan LLM providers
    println!("\n{} LLM Providers:", "[*]".blue().bold());

    let provider_types = vec![
        ProviderType::Copilot,
        ProviderType::Cursor,
        ProviderType::Ollama,
        ProviderType::Vllm,
        ProviderType::Foundry,
        ProviderType::LmStudio,
        ProviderType::LocalAI,
        ProviderType::TextGenWebUI,
        ProviderType::Jan,
        ProviderType::Gpt4All,
        ProviderType::Llamafile,
        ProviderType::OpenAI,
        ProviderType::ChatGPT,
        ProviderType::Anthropic,
        ProviderType::Perplexity,
        ProviderType::DeepSeek,
        ProviderType::Gemini,
    ];

    for pt in &provider_types {
        if let Some(provider) = registry.get_provider(*pt) {
            let available = provider.is_available();
            let session_count = if available {
                provider.list_sessions().map(|s| s.len()).unwrap_or(0)
            } else {
                0
            };

            if available {
                available_providers.push((*pt, session_count));
                total_sessions += session_count;

                let status = if session_count > 0 {
                    format!(
                        "{} {} sessions",
                        "+".green(),
                        session_count.to_string().cyan()
                    )
                } else {
                    format!("{} no sessions", "+".green())
                };

                println!(
                    "   {} {}: {}",
                    "[+]".green(),
                    provider.name().bold(),
                    status
                );

                if show_sessions && session_count > 0 {
                    if let Ok(sessions) = provider.list_sessions() {
                        for session in sessions.iter().take(3) {
                            println!("      {} {}", "`".dimmed(), session.title().dimmed());
                        }
                        if sessions.len() > 3 {
                            println!("      {} ... and {} more", "`".dimmed(), sessions.len() - 3);
                        }
                    }
                }

                if let Some(path) = provider.sessions_path() {
                    println!(
                        "      {} {}",
                        "`".dimmed(),
                        path.display().to_string().dimmed()
                    );
                }
            }
        }
    }

    // Scan VS Code workspaces
    println!("\n{} VS Code Workspaces:", "[*]".blue().bold());

    match discover_workspaces() {
        Ok(workspaces) => {
            let workspaces_with_sessions: Vec<_> = workspaces
                .iter()
                .filter(|ws| ws.chat_session_count > 0)
                .collect();

            let ws_sessions: usize = workspaces_with_sessions
                .iter()
                .map(|ws| ws.chat_session_count)
                .sum();

            println!(
                "   {} {} workspaces with {} sessions",
                "[+]".green(),
                workspaces_with_sessions.len().to_string().cyan(),
                ws_sessions.to_string().cyan()
            );

            if show_sessions {
                for ws in workspaces_with_sessions.iter().take(5) {
                    let name = ws
                        .project_path
                        .clone()
                        .unwrap_or_else(|| ws.hash[..8.min(ws.hash.len())].to_string());
                    println!(
                        "      {} {} ({} sessions)",
                        "`".dimmed(),
                        name.dimmed(),
                        ws.chat_session_count
                    );
                }
                if workspaces_with_sessions.len() > 5 {
                    println!(
                        "      {} ... and {} more workspaces",
                        "`".dimmed(),
                        workspaces_with_sessions.len() - 5
                    );
                }
            }

            total_sessions += ws_sessions;
        }
        Err(e) => {
            println!("   {} Failed to scan workspaces: {}", "[!]".yellow(), e);
        }
    }

    // Scan web-based LLM providers if requested
    let mut web_providers_found = Vec::new();
    let mut authenticated_count = 0;
    if scan_web {
        // First check browser authentication (no network requests)
        println!("\n{} Browser Authentication:", "[*]".blue().bold());
        let (auth_results, auth_count) = scan_browser_authentication(verbose);
        authenticated_count = auth_count;

        // Then probe web endpoints
        println!("\n{} Web LLM Provider Endpoints:", "[*]".blue().bold());
        web_providers_found = scan_web_providers(timeout_secs);

        // Show which authenticated providers are reachable
        if !auth_results.is_empty() {
            println!("\n{} Authenticated Provider Sessions:", "[*]".blue().bold());
            for (provider, browsers) in &auth_results {
                let browser_list: Vec<_> = browsers.iter().map(|b| b.name()).collect();
                let reachable = web_providers_found.iter().any(|p| p == provider);
                let status = if reachable {
                    format!("{} (reachable)", "ready to harvest".green())
                } else {
                    format!("{}", "authenticated but endpoint blocked".yellow())
                };
                println!(
                    "   {} {}: {} via {}",
                    "[+]".green(),
                    provider.bold(),
                    status,
                    browser_list.join(", ").dimmed()
                );
            }
        }
    }

    // Summary
    println!("\n{} Summary:", "[*]".green().bold());
    println!(
        "   {} local providers available",
        available_providers.len().to_string().cyan()
    );
    if scan_web {
        println!(
            "   {} web providers reachable",
            web_providers_found.len().to_string().cyan()
        );
        println!(
            "   {} web providers authenticated",
            authenticated_count.to_string().cyan()
        );
    }
    println!(
        "   {} total sessions to harvest",
        total_sessions.to_string().cyan()
    );

    Ok(())
}

/// Run the harvest operation
pub fn harvest_run(
    path: Option<&str>,
    providers: Option<&[String]>,
    exclude: Option<&[String]>,
    incremental: bool,
    auto_commit: bool,
    message: Option<&str>,
) -> Result<()> {
    let db_path = get_db_path(path)?;

    println!("\n{} Running Harvest", "[H]".magenta().bold());
    println!("{}", "=".repeat(60));

    // Ensure database exists
    if !db_path.exists() {
        println!("{} Database not found, creating...", "[*]".blue());
        create_harvest_database(&db_path)?;
    }

    let conn = Connection::open(&db_path)?;
    let mut stats = HarvestStats::default();

    // Get last harvest time for incremental updates
    let last_harvest: Option<i64> = if incremental {
        conn.query_row("SELECT MAX(harvested_at) FROM sessions", [], |row| {
            row.get(0)
        })
        .ok()
    } else {
        None
    };

    if let Some(ts) = last_harvest {
        let dt = DateTime::from_timestamp_millis(ts)
            .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_default();
        println!("{} Incremental harvest since: {}", "[*]".blue(), dt);
    }

    let registry = ProviderRegistry::new();
    let include_providers = providers.map(|p| p.to_vec());
    let exclude_providers = exclude.map(|p| p.to_vec()).unwrap_or_default();

    // Harvest from LLM providers
    println!("\n{} Harvesting from providers...", "[*]".blue());

    let provider_types = vec![
        ProviderType::Copilot,
        ProviderType::Cursor,
        ProviderType::Ollama,
        ProviderType::Vllm,
        ProviderType::Foundry,
        ProviderType::LmStudio,
        ProviderType::LocalAI,
        ProviderType::TextGenWebUI,
        ProviderType::Jan,
        ProviderType::Gpt4All,
        ProviderType::Llamafile,
    ];

    for pt in &provider_types {
        let provider_name = pt.display_name().to_lowercase();

        // Check include/exclude filters
        if let Some(ref include) = include_providers {
            if !include
                .iter()
                .any(|p| provider_name.contains(&p.to_lowercase()))
            {
                continue;
            }
        }
        if exclude_providers
            .iter()
            .any(|p| provider_name.contains(&p.to_lowercase()))
        {
            continue;
        }

        if let Some(provider) = registry.get_provider(*pt) {
            if !provider.is_available() {
                continue;
            }

            stats.providers_scanned += 1;

            match provider.list_sessions() {
                Ok(sessions) => {
                    for session in sessions {
                        stats.sessions_found += 1;

                        // Check if session should be skipped (incremental)
                        if let Some(last) = last_harvest {
                            if session.last_message_date <= last {
                                stats.sessions_skipped += 1;
                                continue;
                            }
                        }

                        match insert_or_update_session(
                            &conn,
                            &session,
                            pt.display_name(),
                            None,
                            None,
                        ) {
                            Ok(updated) => {
                                if updated {
                                    stats.sessions_updated += 1;
                                } else {
                                    stats.sessions_added += 1;
                                }
                            }
                            Err(e) => {
                                stats.errors.push(format!("{}: {}", session.title(), e));
                            }
                        }
                    }

                    let session_count = stats.sessions_added + stats.sessions_updated;
                    if session_count > 0 {
                        println!(
                            "   {} {}: {} sessions",
                            "[+]".green(),
                            provider.name(),
                            session_count.to_string().cyan()
                        );
                    }
                }
                Err(e) => {
                    stats.errors.push(format!("{}: {}", provider.name(), e));
                }
            }
        }
    }

    // Harvest from VS Code workspaces
    if include_providers.is_none()
        || include_providers
            .as_ref()
            .map(|p| {
                p.iter()
                    .any(|x| x == "copilot" || x == "vscode" || x == "workspace")
            })
            .unwrap_or(false)
    {
        println!("\n{} Harvesting from VS Code workspaces...", "[*]".blue());

        if let Ok(workspaces) = discover_workspaces() {
            for ws in &workspaces {
                if ws.chat_session_count == 0 {
                    continue;
                }

                stats.workspaces_scanned += 1;

                if let Ok(sessions) = get_chat_sessions_from_workspace(&ws.workspace_path) {
                    for swp in sessions {
                        stats.sessions_found += 1;

                        // Check if session should be skipped (incremental)
                        if let Some(last) = last_harvest {
                            if swp.session.last_message_date <= last {
                                stats.sessions_skipped += 1;
                                continue;
                            }
                        }

                        let ws_name = ws.project_path.clone();

                        match insert_or_update_session(
                            &conn,
                            &swp.session,
                            "GitHub Copilot",
                            Some(&ws.hash),
                            ws_name.as_deref(),
                        ) {
                            Ok(updated) => {
                                if updated {
                                    stats.sessions_updated += 1;
                                } else {
                                    stats.sessions_added += 1;
                                }
                            }
                            Err(e) => {
                                stats.errors.push(format!("{}: {}", swp.session.title(), e));
                            }
                        }
                    }
                }
            }

            println!(
                "   {} Workspaces: {} scanned",
                "[+]".green(),
                stats.workspaces_scanned.to_string().cyan()
            );
        }
    }

    // Harvest from web-based cloud providers (ChatGPT, Claude, etc.)
    let include_list: Vec<String> = include_providers.clone().unwrap_or_default();
    harvest_web_providers(&conn, &mut stats, &include_list, &exclude_providers)?;

    // Update metadata
    update_harvest_metadata(&conn)?;

    // Print summary
    println!("\n{} Harvest Complete:", "[+]".green().bold());
    println!(
        "   {} providers scanned",
        stats.providers_scanned.to_string().cyan()
    );
    println!(
        "   {} workspaces scanned",
        stats.workspaces_scanned.to_string().cyan()
    );
    println!(
        "   {} sessions found",
        stats.sessions_found.to_string().cyan()
    );
    println!(
        "   {} sessions added",
        stats.sessions_added.to_string().green()
    );
    println!(
        "   {} sessions updated",
        stats.sessions_updated.to_string().yellow()
    );
    if stats.sessions_skipped > 0 {
        println!(
            "   {} sessions skipped (unchanged)",
            stats.sessions_skipped.to_string().dimmed()
        );
    }

    if !stats.errors.is_empty() {
        println!("\n{} Errors ({}):", "[!]".red(), stats.errors.len());
        for (i, err) in stats.errors.iter().take(5).enumerate() {
            println!("   {}. {}", i + 1, err);
        }
        if stats.errors.len() > 5 {
            println!("   ... and {} more errors", stats.errors.len() - 5);
        }
    }

    // Auto-commit if requested
    if auto_commit && (stats.sessions_added > 0 || stats.sessions_updated > 0) {
        println!("\n{} Auto-committing changes...", "[*]".blue());
        let commit_msg = message.unwrap_or("Harvest: update chat sessions");
        if let Err(e) = git_commit_harvest(&db_path, commit_msg) {
            println!("{} Git commit failed: {}", "[!]".yellow(), e);
        } else {
            println!("{} Changes committed", "[+]".green());
        }
    }

    println!("\nDatabase: {}", db_path.display());

    Ok(())
}

/// Show harvest database status
pub fn harvest_status(path: Option<&str>) -> Result<()> {
    let db_path = get_db_path(path)?;

    println!("\n{} Harvest Database Status", "[H]".magenta().bold());
    println!("{}", "=".repeat(60));

    if !db_path.exists() {
        println!(
            "{} Database not found: {}",
            "[!]".yellow(),
            db_path.display()
        );
        println!("   Run 'csm harvest init' to create one");
        return Ok(());
    }

    let conn = Connection::open(&db_path)?;

    // Get session counts by provider
    let mut stmt = conn.prepare(
        "SELECT provider, COUNT(*) as count, SUM(message_count) as messages 
         FROM sessions 
         GROUP BY provider 
         ORDER BY count DESC",
    )?;

    let provider_stats: Vec<(String, i64, i64)> = stmt
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get::<_, Option<i64>>(2)?.unwrap_or(0),
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    // Get total counts
    let total_sessions: i64 = conn
        .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
        .unwrap_or(0);

    let total_messages: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(message_count), 0) FROM sessions",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // Get last harvest time
    let last_harvest: Option<i64> = conn
        .query_row("SELECT MAX(harvested_at) FROM sessions", [], |row| {
            row.get(0)
        })
        .ok()
        .flatten();

    // Get oldest and newest sessions
    let oldest: Option<i64> = conn
        .query_row("SELECT MIN(created_at) FROM sessions", [], |row| row.get(0))
        .ok()
        .flatten();

    let newest: Option<i64> = conn
        .query_row("SELECT MAX(updated_at) FROM sessions", [], |row| row.get(0))
        .ok()
        .flatten();

    println!("{} Database: {}", "[*]".blue(), db_path.display());
    println!(
        "{} Total Sessions: {}",
        "[*]".blue(),
        total_sessions.to_string().cyan()
    );
    println!(
        "{} Total Messages: {}",
        "[*]".blue(),
        total_messages.to_string().cyan()
    );

    if let Some(ts) = last_harvest {
        let dt = DateTime::from_timestamp_millis(ts)
            .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        println!("{} Last Harvest: {}", "[*]".blue(), dt);
    }

    if let (Some(old), Some(new)) = (oldest, newest) {
        let old_dt = DateTime::from_timestamp_millis(old)
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default();
        let new_dt = DateTime::from_timestamp_millis(new)
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default();
        println!("{} Date Range: {} to {}", "[*]".blue(), old_dt, new_dt);
    }

    if !provider_stats.is_empty() {
        println!("\n{} Sessions by Provider:", "[*]".blue());
        for (provider, count, messages) in &provider_stats {
            println!(
                "   {} {}: {} sessions, {} messages",
                "[+]".green(),
                provider.bold(),
                count.to_string().cyan(),
                messages.to_string().dimmed()
            );
        }
    }

    // Check git status
    let db_dir = db_path.parent().unwrap_or(Path::new("."));
    if db_dir.join(".git").exists()
        || db_path
            .parent()
            .map(|p| p.join(".git").exists())
            .unwrap_or(false)
    {
        println!("\n{} Git Status:", "[*]".blue());

        let output = Command::new("git")
            .current_dir(db_dir)
            .args([
                "status",
                "--porcelain",
                db_path.file_name().unwrap().to_str().unwrap(),
            ])
            .output();

        match output {
            Ok(out) => {
                let status = String::from_utf8_lossy(&out.stdout);
                if status.is_empty() {
                    println!("   {} No uncommitted changes", "[+]".green());
                } else {
                    println!("   {} Uncommitted changes detected", "[!]".yellow());
                    println!("   Run 'csm harvest git commit' to save changes");
                }
            }
            Err(_) => {
                println!("   {} Unable to check git status", "[!]".yellow());
            }
        }
    } else {
        println!("\n{} Git tracking not enabled", "[i]".dimmed());
        println!("   Run 'csm harvest git init' to enable version tracking");
    }

    Ok(())
}

/// List sessions in the harvest database
pub fn harvest_list(
    path: Option<&str>,
    provider: Option<&str>,
    limit: usize,
    search: Option<&str>,
) -> Result<()> {
    let db_path = get_db_path(path)?;

    if !db_path.exists() {
        println!(
            "{} Database not found: {}",
            "[!]".yellow(),
            db_path.display()
        );
        return Ok(());
    }

    let conn = Connection::open(&db_path)?;

    let mut query = String::from(
        "SELECT id, provider, title, message_count, created_at, updated_at, workspace_name 
         FROM sessions WHERE 1=1",
    );
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(p) = provider {
        query.push_str(" AND LOWER(provider) LIKE ?");
        params_vec.push(Box::new(format!("%{}%", p.to_lowercase())));
    }

    if let Some(s) = search {
        query.push_str(" AND (LOWER(title) LIKE ? OR LOWER(id) LIKE ?)");
        let pattern = format!("%{}%", s.to_lowercase());
        params_vec.push(Box::new(pattern.clone()));
        params_vec.push(Box::new(pattern));
    }

    query.push_str(" ORDER BY updated_at DESC LIMIT ?");
    params_vec.push(Box::new(limit as i64));

    let mut stmt = conn.prepare(&query)?;

    // Build params slice
    let params_slice: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();

    let sessions: Vec<HarvestQueryResult> = stmt
        .query_map(params_slice.as_slice(), |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    println!("\n{} Harvested Sessions", "[H]".magenta().bold());
    println!("{}", "=".repeat(60));

    if sessions.is_empty() {
        println!("{} No sessions found", "[i]".dimmed());
        return Ok(());
    }

    for (id, prov, title, msg_count, _created, updated, ws_name) in &sessions {
        let date = DateTime::from_timestamp_millis(*updated)
            .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_default();

        println!("\n{} {}", "[S]".cyan(), title.bold());
        println!("   ID: {}", &id[..16.min(id.len())].dimmed());
        println!(
            "   Provider: {} | Messages: {} | Updated: {}",
            prov.cyan(),
            msg_count.to_string().green(),
            date.dimmed()
        );
        if let Some(ws) = ws_name {
            println!("   Workspace: {}", ws.dimmed());
        }
    }

    println!(
        "\n{} Showing {} of available sessions",
        "[i]".dimmed(),
        sessions.len()
    );

    Ok(())
}

/// Export sessions from the harvest database
pub fn harvest_export(
    path: Option<&str>,
    output: &str,
    format: &str,
    provider: Option<&str>,
    session_ids: Option<&[String]>,
) -> Result<()> {
    let db_path = get_db_path(path)?;
    let output_path = PathBuf::from(output);

    if !db_path.exists() {
        anyhow::bail!("Database not found: {}", db_path.display());
    }

    let conn = Connection::open(&db_path)?;

    println!("\n{} Exporting Sessions", "[H]".magenta().bold());
    println!("{}", "=".repeat(60));

    // Build query
    let mut query = String::from("SELECT session_json FROM sessions WHERE 1=1");
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(p) = provider {
        query.push_str(" AND LOWER(provider) LIKE ?");
        params_vec.push(Box::new(format!("%{}%", p.to_lowercase())));
    }

    if let Some(ids) = session_ids {
        let placeholders: Vec<&str> = ids.iter().map(|_| "?").collect();
        query.push_str(&format!(" AND id IN ({})", placeholders.join(",")));
        for id in ids {
            params_vec.push(Box::new(id.clone()));
        }
    }

    let mut stmt = conn.prepare(&query)?;
    let params_slice: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();

    let sessions: Vec<String> = stmt
        .query_map(params_slice.as_slice(), |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    if sessions.is_empty() {
        println!("{} No sessions to export", "[i]".dimmed());
        return Ok(());
    }

    // Create output directory if needed
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    match format.to_lowercase().as_str() {
        "json" => {
            // Export as JSON array
            let parsed: Vec<serde_json::Value> = sessions
                .iter()
                .filter_map(|s| serde_json::from_str(s).ok())
                .collect();
            let json_output = serde_json::to_string_pretty(&parsed)?;
            fs::write(&output_path, json_output)?;
        }
        "jsonl" => {
            // Export as JSON Lines
            let content: String = sessions
                .iter()
                .filter_map(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                .map(|v| serde_json::to_string(&v).unwrap_or_default())
                .collect::<Vec<_>>()
                .join("\n");
            fs::write(&output_path, content)?;
        }
        "md" | "markdown" => {
            // Export as Markdown
            let mut md_content = String::from("# Chat Sessions Export\n\n");
            md_content.push_str(&format!(
                "Exported: {}\n\n",
                Utc::now().format("%Y-%m-%d %H:%M:%S")
            ));

            for session_json in &sessions {
                if let Ok(session) = parse_session_json(session_json) {
                    md_content.push_str(&format!("## {}\n\n", session.title()));
                    md_content.push_str(&format!("Messages: {}\n\n", session.request_count()));

                    for request in &session.requests {
                        if let Some(msg) = &request.message {
                            if let Some(text) = &msg.text {
                                md_content.push_str(&format!("### User\n\n{}\n\n", text));
                            }
                        }
                        // Extract response text from the JSON value
                        if let Some(response) = &request.response {
                            let response_text = response
                                .get("text")
                                .and_then(|v| v.as_str())
                                .or_else(|| response.get("content").and_then(|v| v.as_str()))
                                .or_else(|| {
                                    // Try to get from value array (older format)
                                    response
                                        .get("value")
                                        .and_then(|v| v.as_array())
                                        .and_then(|arr| arr.first())
                                        .and_then(|v| v.get("value"))
                                        .and_then(|v| v.as_str())
                                });

                            if let Some(text) = response_text {
                                md_content.push_str(&format!("### Assistant\n\n{}\n\n", text));
                            }
                        }
                    }
                    md_content.push_str("---\n\n");
                }
            }
            fs::write(&output_path, md_content)?;
        }
        _ => {
            anyhow::bail!("Unknown format: {}. Supported: json, jsonl, md", format);
        }
    }

    println!(
        "{} Exported {} sessions to {}",
        "[+]".green(),
        sessions.len().to_string().cyan(),
        output_path.display()
    );

    Ok(())
}

/// Git operations for harvest database
pub fn harvest_git_init(path: Option<&str>) -> Result<()> {
    let db_path = get_db_path(path)?;
    init_git_tracking(&db_path)
}

pub fn harvest_git_commit(path: Option<&str>, message: Option<&str>) -> Result<()> {
    let db_path = get_db_path(path)?;
    let msg = message.unwrap_or("Update harvest database");
    git_commit_harvest(&db_path, msg)
}

pub fn harvest_git_log(path: Option<&str>, count: usize) -> Result<()> {
    let db_path = get_db_path(path)?;
    let db_dir = db_path.parent().unwrap_or(Path::new("."));
    let db_name = db_path.file_name().unwrap().to_str().unwrap();

    println!("\n{} Harvest Git History", "[H]".magenta().bold());
    println!("{}", "=".repeat(60));

    let output = Command::new("git")
        .current_dir(db_dir)
        .args([
            "log",
            "--oneline",
            &format!("-{}", count),
            "--follow",
            "--",
            db_name,
        ])
        .output()
        .context("Failed to run git log")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("does not have any commits") {
            println!("{} No commits yet", "[i]".dimmed());
        } else {
            anyhow::bail!("Git log failed: {}", stderr);
        }
        return Ok(());
    }

    let log = String::from_utf8_lossy(&output.stdout);
    if log.trim().is_empty() {
        println!("{} No commits found for {}", "[i]".dimmed(), db_name);
    } else {
        for line in log.lines() {
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            if parts.len() == 2 {
                println!("{} {} {}", "[C]".yellow(), parts[0].cyan(), parts[1]);
            } else {
                println!("{}", line);
            }
        }
    }

    Ok(())
}

pub fn harvest_git_diff(path: Option<&str>, commit: Option<&str>) -> Result<()> {
    let db_path = get_db_path(path)?;
    let db_dir = db_path.parent().unwrap_or(Path::new("."));
    let db_name = db_path.file_name().unwrap().to_str().unwrap();

    println!("\n{} Harvest Database Changes", "[H]".magenta().bold());
    println!("{}", "=".repeat(60));

    let mut args = vec!["diff", "--stat"];
    if let Some(c) = commit {
        args.push(c);
    }
    args.push("--");
    args.push(db_name);

    let output = Command::new("git")
        .current_dir(db_dir)
        .args(&args)
        .output()
        .context("Failed to run git diff")?;

    let diff = String::from_utf8_lossy(&output.stdout);
    if diff.trim().is_empty() {
        println!("{} No changes", "[+]".green());
    } else {
        println!("{}", diff);
    }

    Ok(())
}

pub fn harvest_git_restore(path: Option<&str>, commit: &str) -> Result<()> {
    let db_path = get_db_path(path)?;
    let db_dir = db_path.parent().unwrap_or(Path::new("."));
    let db_name = db_path.file_name().unwrap().to_str().unwrap();

    println!("\n{} Restoring Harvest Database", "[H]".magenta().bold());
    println!("{}", "=".repeat(60));

    // Create backup first
    let backup_path = db_path.with_extension("db.backup");
    if db_path.exists() {
        fs::copy(&db_path, &backup_path)?;
        println!(
            "{} Created backup: {}",
            "[+]".green(),
            backup_path.display()
        );
    }

    // Restore from commit
    let output = Command::new("git")
        .current_dir(db_dir)
        .args(["checkout", commit, "--", db_name])
        .output()
        .context("Failed to run git checkout")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Git restore failed: {}", stderr);
    }

    println!(
        "{} Restored database from commit: {}",
        "[+]".green(),
        commit
    );

    Ok(())
}

// ============================================================================
// Helper Functions
// ============================================================================

fn get_db_path(path: Option<&str>) -> Result<PathBuf> {
    if let Some(p) = path {
        return Ok(PathBuf::from(p));
    }

    // Check environment variable
    if let Ok(p) = std::env::var("CSM_HARVEST_DB") {
        return Ok(PathBuf::from(p));
    }

    // Default to current directory
    Ok(std::env::current_dir()?.join("chat_sessions.db"))
}

fn create_harvest_database(path: &Path) -> Result<()> {
    let conn = Connection::open(path)?;

    conn.execute_batch(
        r#"
        -- Sessions table (original harvest format)
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            provider TEXT NOT NULL,
            provider_type TEXT,
            workspace_id TEXT,
            workspace_name TEXT,
            title TEXT NOT NULL,
            message_count INTEGER DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            harvested_at INTEGER NOT NULL,
            session_json TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_sessions_provider ON sessions(provider);
        CREATE INDEX IF NOT EXISTS idx_sessions_workspace ON sessions(workspace_id);
        CREATE INDEX IF NOT EXISTS idx_sessions_updated ON sessions(updated_at);
        
        -- Enhanced messages table with raw markdown and metadata
        CREATE TABLE IF NOT EXISTS messages_v2 (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            message_index INTEGER NOT NULL,
            request_id TEXT,
            response_id TEXT,
            role TEXT NOT NULL,
            content_raw TEXT NOT NULL,
            content_markdown TEXT,
            model_id TEXT,
            timestamp INTEGER,
            is_canceled INTEGER DEFAULT 0,
            metadata_json TEXT,
            created_at INTEGER DEFAULT (strftime('%s', 'now') * 1000),
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
            UNIQUE(session_id, message_index, role)
        );
        
        CREATE INDEX IF NOT EXISTS idx_messages_v2_session ON messages_v2(session_id);
        CREATE INDEX IF NOT EXISTS idx_messages_v2_role ON messages_v2(role);
        CREATE INDEX IF NOT EXISTS idx_messages_v2_timestamp ON messages_v2(timestamp);
        
        -- Tool invocations within messages (file edits, terminal commands, etc.)
        CREATE TABLE IF NOT EXISTS tool_invocations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            message_id INTEGER NOT NULL,
            session_id TEXT NOT NULL,
            tool_name TEXT NOT NULL,
            tool_call_id TEXT,
            invocation_index INTEGER DEFAULT 0,
            input_json TEXT,
            output_json TEXT,
            status TEXT DEFAULT 'pending',
            is_confirmed INTEGER DEFAULT 0,
            timestamp INTEGER,
            FOREIGN KEY (message_id) REFERENCES messages_v2(id) ON DELETE CASCADE,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
        );
        
        CREATE INDEX IF NOT EXISTS idx_tool_invocations_message ON tool_invocations(message_id);
        CREATE INDEX IF NOT EXISTS idx_tool_invocations_session ON tool_invocations(session_id);
        CREATE INDEX IF NOT EXISTS idx_tool_invocations_tool ON tool_invocations(tool_name);
        
        -- File changes/diffs associated with tool invocations
        CREATE TABLE IF NOT EXISTS file_changes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tool_invocation_id INTEGER,
            session_id TEXT NOT NULL,
            message_index INTEGER,
            file_path TEXT NOT NULL,
            change_type TEXT NOT NULL,
            old_content TEXT,
            new_content TEXT,
            diff_unified TEXT,
            line_start INTEGER,
            line_end INTEGER,
            timestamp INTEGER,
            FOREIGN KEY (tool_invocation_id) REFERENCES tool_invocations(id) ON DELETE CASCADE,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
        );
        
        CREATE INDEX IF NOT EXISTS idx_file_changes_tool ON file_changes(tool_invocation_id);
        CREATE INDEX IF NOT EXISTS idx_file_changes_session ON file_changes(session_id);
        CREATE INDEX IF NOT EXISTS idx_file_changes_path ON file_changes(file_path);
        
        -- Message-level checkpoints for versioning
        CREATE TABLE IF NOT EXISTS message_checkpoints (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            message_index INTEGER NOT NULL,
            checkpoint_number INTEGER NOT NULL,
            content_hash TEXT NOT NULL,
            snapshot_json TEXT,
            file_state_json TEXT,
            created_at INTEGER DEFAULT (strftime('%s', 'now') * 1000),
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
            UNIQUE(session_id, message_index, checkpoint_number)
        );
        
        CREATE INDEX IF NOT EXISTS idx_message_checkpoints_session ON message_checkpoints(session_id);
        CREATE INDEX IF NOT EXISTS idx_sessions_title ON sessions(title);

        -- Messages table for full-text search
        CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            message_index INTEGER NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at INTEGER,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
            UNIQUE(session_id, message_index)
        );
        
        CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id);
        CREATE INDEX IF NOT EXISTS idx_messages_role ON messages(role);
        
        -- Checkpoints for version tracking
        CREATE TABLE IF NOT EXISTS checkpoints (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            checkpoint_number INTEGER NOT NULL,
            message TEXT,
            message_count INTEGER NOT NULL,
            content_hash TEXT NOT NULL,
            snapshot TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
            UNIQUE(session_id, checkpoint_number)
        );
        
        CREATE INDEX IF NOT EXISTS idx_checkpoints_session ON checkpoints(session_id);
        
        -- Share links for importing shared conversations
        CREATE TABLE IF NOT EXISTS share_links (
            id TEXT PRIMARY KEY,
            session_id TEXT,
            provider TEXT NOT NULL,
            url TEXT NOT NULL UNIQUE,
            share_id TEXT NOT NULL,
            title TEXT,
            imported INTEGER DEFAULT 0,
            imported_at INTEGER,
            created_at INTEGER NOT NULL,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE SET NULL
        );
        
        CREATE INDEX IF NOT EXISTS idx_share_links_provider ON share_links(provider);
        CREATE INDEX IF NOT EXISTS idx_share_links_imported ON share_links(imported);

        -- Harvest metadata
        CREATE TABLE IF NOT EXISTS harvest_metadata (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        INSERT OR REPLACE INTO harvest_metadata (key, value) 
        VALUES ('version', '2.1'),
               ('created_at', datetime('now'));
               
        -- Full-text search for messages (standalone FTS table)
        CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
            content_raw
        );
        
        -- Triggers to keep FTS index in sync with messages_v2
        CREATE TRIGGER IF NOT EXISTS messages_v2_ai AFTER INSERT ON messages_v2 BEGIN
            INSERT INTO messages_fts(rowid, content_raw) VALUES (new.id, new.content_raw);
        END;
        
        CREATE TRIGGER IF NOT EXISTS messages_v2_ad AFTER DELETE ON messages_v2 BEGIN
            INSERT INTO messages_fts(messages_fts, rowid, content_raw) VALUES ('delete', old.id, old.content_raw);
        END;
        
        CREATE TRIGGER IF NOT EXISTS messages_v2_au AFTER UPDATE ON messages_v2 BEGIN
            INSERT INTO messages_fts(messages_fts, rowid, content_raw) VALUES ('delete', old.id, old.content_raw);
            INSERT INTO messages_fts(rowid, content_raw) VALUES (new.id, new.content_raw);
        END;
        "#,
    )?;

    Ok(())
}

fn insert_or_update_session(
    conn: &Connection,
    session: &ChatSession,
    provider: &str,
    workspace_id: Option<&str>,
    workspace_name: Option<&str>,
) -> Result<bool> {
    let session_id = session
        .session_id
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let now = Utc::now().timestamp_millis();
    let session_json = serde_json::to_string(session)?;

    // Check if session exists
    let existing: Option<i64> = conn
        .query_row(
            "SELECT updated_at FROM sessions WHERE id = ?",
            [&session_id],
            |row| row.get(0),
        )
        .ok();

    let updated = existing.is_some();

    conn.execute(
        r#"
        INSERT OR REPLACE INTO sessions 
        (id, provider, provider_type, workspace_id, workspace_name, title, 
         message_count, created_at, updated_at, harvested_at, session_json)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        params![
            session_id,
            provider,
            provider,
            workspace_id,
            workspace_name,
            session.title(),
            session.request_count() as i64,
            session.creation_date,
            session.last_message_date,
            now,
            session_json,
        ],
    )?;

    // Populate enhanced message tables
    populate_enhanced_messages(conn, &session_id, session)?;

    Ok(updated)
}

/// Populate the enhanced messages_v2, tool_invocations, and file_changes tables
fn populate_enhanced_messages(
    conn: &Connection,
    session_id: &str,
    session: &ChatSession,
) -> Result<()> {
    // Delete existing messages for this session to avoid duplicates
    conn.execute("DELETE FROM messages_v2 WHERE session_id = ?", [session_id])?;
    conn.execute(
        "DELETE FROM tool_invocations WHERE session_id = ?",
        [session_id],
    )?;
    conn.execute(
        "DELETE FROM file_changes WHERE session_id = ?",
        [session_id],
    )?;

    for (idx, request) in session.requests.iter().enumerate() {
        let timestamp = request.timestamp;
        let request_id = request.request_id.as_deref();
        let response_id = request.response_id.as_deref();
        let model_id = request.model_id.as_deref();
        let is_canceled = request.is_canceled.unwrap_or(false);

        // Insert user message
        if let Some(ref message) = request.message {
            let content = message.text.clone().unwrap_or_default();
            if !content.is_empty() {
                let metadata = serde_json::json!({
                    "variable_data": request.variable_data,
                });

                conn.execute(
                    r#"
                    INSERT OR REPLACE INTO messages_v2 
                    (session_id, message_index, request_id, response_id, role, 
                     content_raw, content_markdown, model_id, timestamp, is_canceled, metadata_json)
                    VALUES (?, ?, ?, ?, 'user', ?, ?, ?, ?, 0, ?)
                    "#,
                    params![
                        session_id,
                        (idx * 2) as i64,
                        request_id,
                        response_id,
                        &content,
                        &content, // content_markdown same as raw for user messages
                        model_id,
                        timestamp,
                        serde_json::to_string(&metadata).ok(),
                    ],
                )?;
            }
        }

        // Insert assistant response with tool invocations
        if let Some(ref response) = request.response {
            let (content, tool_invocations) = extract_response_content_and_tools(response);

            if !content.is_empty() || !tool_invocations.is_empty() {
                let metadata = serde_json::json!({
                    "content_references": request.content_references,
                    "code_citations": request.code_citations,
                    "response_markdown_info": request.response_markdown_info,
                });

                conn.execute(
                    r#"
                    INSERT OR REPLACE INTO messages_v2 
                    (session_id, message_index, request_id, response_id, role, 
                     content_raw, content_markdown, model_id, timestamp, is_canceled, metadata_json)
                    VALUES (?, ?, ?, ?, 'assistant', ?, ?, ?, ?, ?, ?)
                    "#,
                    params![
                        session_id,
                        (idx * 2 + 1) as i64,
                        request_id,
                        response_id,
                        &content,
                        &content,
                        model_id,
                        timestamp,
                        is_canceled as i64,
                        serde_json::to_string(&metadata).ok(),
                    ],
                )?;

                // Get the message_id we just inserted
                let message_id: i64 = conn.last_insert_rowid();

                // Insert tool invocations and file changes
                for (inv_idx, invocation) in tool_invocations.iter().enumerate() {
                    insert_tool_invocation(
                        conn, message_id, session_id, inv_idx, invocation, timestamp,
                    )?;
                }
            }
        }
    }

    Ok(())
}

/// Extract response content and tool invocations from the response JSON
fn extract_response_content_and_tools(
    response: &serde_json::Value,
) -> (String, Vec<serde_json::Value>) {
    let mut text_parts = Vec::new();
    let mut tool_invocations = Vec::new();

    if let Some(items) = response.as_array() {
        for item in items {
            let kind = item.get("kind").and_then(|k| k.as_str()).unwrap_or("");

            match kind {
                "toolInvocationSerialized" => {
                    tool_invocations.push(item.clone());
                }
                "thinking" => {
                    // Skip thinking blocks
                    continue;
                }
                _ => {
                    if let Some(value) = item.get("value").and_then(|v| v.as_str()) {
                        // Filter out empty code block markers (```\n or just ```)
                        let trimmed = value.trim();
                        if !trimmed.is_empty() && !is_empty_code_block(trimmed) {
                            text_parts.push(value.to_string());
                        }
                    }
                }
            }
        }
    }

    (text_parts.join("\n\n"), tool_invocations)
}

/// Insert a tool invocation and any associated file changes
fn insert_tool_invocation(
    conn: &Connection,
    message_id: i64,
    session_id: &str,
    inv_idx: usize,
    invocation: &serde_json::Value,
    timestamp: Option<i64>,
) -> Result<()> {
    let tool_name = invocation
        .get("toolId")
        .and_then(|t| t.as_str())
        .unwrap_or("unknown");
    let tool_call_id = invocation.get("toolCallId").and_then(|t| t.as_str());
    let is_complete = invocation
        .get("isComplete")
        .and_then(|c| c.as_bool())
        .unwrap_or(false);
    let is_confirmed = invocation.get("isConfirmed");
    let tool_data = invocation.get("toolSpecificData");

    let input_json = tool_data.map(|d| serde_json::to_string(d).unwrap_or_default());
    let status = if is_complete { "complete" } else { "pending" };
    let confirmed = match is_confirmed {
        Some(v) => v
            .get("type")
            .and_then(|t| t.as_i64())
            .map(|t| t > 0)
            .unwrap_or(false),
        None => false,
    };

    conn.execute(
        r#"
        INSERT INTO tool_invocations 
        (message_id, session_id, tool_name, tool_call_id, invocation_index, 
         input_json, status, is_confirmed, timestamp)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        params![
            message_id,
            session_id,
            tool_name,
            tool_call_id,
            inv_idx as i64,
            input_json,
            status,
            confirmed as i64,
            timestamp,
        ],
    )?;

    let tool_invocation_id = conn.last_insert_rowid();

    // Extract and insert file changes based on tool type
    if let Some(data) = tool_data {
        insert_file_changes(conn, tool_invocation_id, session_id, data, timestamp)?;
    }

    Ok(())
}

/// Insert file changes from tool-specific data
fn insert_file_changes(
    conn: &Connection,
    tool_invocation_id: i64,
    session_id: &str,
    tool_data: &serde_json::Value,
    timestamp: Option<i64>,
) -> Result<()> {
    let kind = tool_data.get("kind").and_then(|k| k.as_str()).unwrap_or("");

    match kind {
        "terminal" => {
            // Terminal command execution
            if let Some(cmd_line) = tool_data.get("commandLine") {
                let original = cmd_line.get("original").and_then(|o| o.as_str());
                let edited = cmd_line.get("toolEdited").and_then(|e| e.as_str());
                let output = tool_data
                    .get("terminalCommandOutput")
                    .map(|o| serde_json::to_string(o).unwrap_or_default());

                conn.execute(
                    r#"
                    INSERT INTO file_changes 
                    (tool_invocation_id, session_id, file_path, change_type, 
                     old_content, new_content, diff_unified, timestamp)
                    VALUES (?, ?, '[terminal]', 'command', ?, ?, ?, ?)
                    "#,
                    params![
                        tool_invocation_id,
                        session_id,
                        original,
                        edited.or(original),
                        output,
                        timestamp,
                    ],
                )?;
            }
        }
        "replaceFile" | "editFile" => {
            // File edit with old/new strings
            let file_path = tool_data
                .get("uri")
                .or_else(|| tool_data.get("filePath"))
                .and_then(|p| p.as_str())
                .unwrap_or("[unknown]");
            let old_string = tool_data.get("oldString").and_then(|s| s.as_str());
            let new_string = tool_data.get("newString").and_then(|s| s.as_str());

            // Generate unified diff if we have both old and new content
            let diff = if let (Some(old), Some(new)) = (old_string, new_string) {
                Some(generate_unified_diff(old, new, file_path))
            } else {
                None
            };

            conn.execute(
                r#"
                INSERT INTO file_changes 
                (tool_invocation_id, session_id, file_path, change_type, 
                 old_content, new_content, diff_unified, timestamp)
                VALUES (?, ?, ?, 'edit', ?, ?, ?, ?)
                "#,
                params![
                    tool_invocation_id,
                    session_id,
                    file_path,
                    old_string,
                    new_string,
                    diff,
                    timestamp,
                ],
            )?;
        }
        "createFile" => {
            let file_path = tool_data
                .get("uri")
                .or_else(|| tool_data.get("filePath"))
                .and_then(|p| p.as_str())
                .unwrap_or("[unknown]");
            let content = tool_data.get("content").and_then(|c| c.as_str());

            conn.execute(
                r#"
                INSERT INTO file_changes 
                (tool_invocation_id, session_id, file_path, change_type, 
                 new_content, timestamp)
                VALUES (?, ?, ?, 'create', ?, ?)
                "#,
                params![
                    tool_invocation_id,
                    session_id,
                    file_path,
                    content,
                    timestamp,
                ],
            )?;
        }
        "readFile" => {
            let file_path = tool_data
                .get("uri")
                .or_else(|| tool_data.get("filePath"))
                .and_then(|p| p.as_str())
                .unwrap_or("[unknown]");

            conn.execute(
                r#"
                INSERT INTO file_changes 
                (tool_invocation_id, session_id, file_path, change_type, timestamp)
                VALUES (?, ?, ?, 'read', ?)
                "#,
                params![tool_invocation_id, session_id, file_path, timestamp,],
            )?;
        }
        _ => {
            // Other tool types - store as generic change
            if !kind.is_empty() {
                let data_json = serde_json::to_string(tool_data).ok();
                conn.execute(
                    r#"
                    INSERT INTO file_changes 
                    (tool_invocation_id, session_id, file_path, change_type, 
                     diff_unified, timestamp)
                    VALUES (?, ?, ?, ?, ?, ?)
                    "#,
                    params![
                        tool_invocation_id,
                        session_id,
                        format!("[{}]", kind),
                        kind,
                        data_json,
                        timestamp,
                    ],
                )?;
            }
        }
    }

    Ok(())
}

/// Generate a simple unified diff between two strings
fn generate_unified_diff(old: &str, new: &str, file_path: &str) -> String {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    let mut diff = format!("--- a/{}\n+++ b/{}\n", file_path, file_path);

    // Simple line-by-line diff (not a full Myers diff, but good enough for storage)
    let max_lines = old_lines.len().max(new_lines.len());
    let mut in_hunk = false;
    let mut hunk_start = 0;
    let mut hunk_lines = Vec::new();

    for i in 0..max_lines {
        let old_line = old_lines.get(i).copied();
        let new_line = new_lines.get(i).copied();

        match (old_line, new_line) {
            (Some(o), Some(n)) if o == n => {
                if in_hunk {
                    hunk_lines.push(format!(" {}", o));
                }
            }
            (Some(o), Some(n)) => {
                if !in_hunk {
                    in_hunk = true;
                    hunk_start = i + 1;
                }
                hunk_lines.push(format!("-{}", o));
                hunk_lines.push(format!("+{}", n));
            }
            (Some(o), None) => {
                if !in_hunk {
                    in_hunk = true;
                    hunk_start = i + 1;
                }
                hunk_lines.push(format!("-{}", o));
            }
            (None, Some(n)) => {
                if !in_hunk {
                    in_hunk = true;
                    hunk_start = i + 1;
                }
                hunk_lines.push(format!("+{}", n));
            }
            (None, None) => break,
        }
    }

    if !hunk_lines.is_empty() {
        diff.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            hunk_start,
            old_lines.len(),
            hunk_start,
            new_lines.len()
        ));
        for line in hunk_lines {
            diff.push_str(&line);
            diff.push('\n');
        }
    }

    diff
}

/// Harvest sessions from web-based cloud providers (ChatGPT, Claude, etc.)
fn harvest_web_providers(
    conn: &Connection,
    stats: &mut HarvestStats,
    include_providers: &[String],
    exclude_providers: &[String],
) -> Result<()> {
    use crate::browser::extract_provider_cookies;

    println!("\n{} Harvesting from web providers...", "[*]".blue());

    // Define web providers to harvest from
    let web_provider_configs: Vec<(&str, &str, &str)> = vec![
        ("ChatGPT", "chatgpt", "__Secure-next-auth.session-token"),
        ("Claude", "claude", "sessionKey"),
    ];

    let mut web_sessions_harvested = 0;

    for (display_name, provider_key, _cookie_name) in &web_provider_configs {
        // Check provider filters
        if !include_providers.is_empty()
            && !include_providers
                .iter()
                .any(|p| p.eq_ignore_ascii_case(provider_key))
        {
            continue;
        }
        if exclude_providers
            .iter()
            .any(|p| p.eq_ignore_ascii_case(provider_key))
        {
            continue;
        }

        print!("   {} Checking {} ... ", "[-]".yellow(), display_name);

        // Try to extract cookies for this provider
        if let Some(creds) = extract_provider_cookies(provider_key) {
            if let Some(session_token) = &creds.session_token {
                println!("{}", "authenticated".green());

                // Create provider and fetch conversations
                let result = match *provider_key {
                    "chatgpt" => harvest_chatgpt_sessions(conn, session_token, stats),
                    "claude" => harvest_claude_sessions(conn, session_token, stats),
                    _ => Ok(0),
                };

                match result {
                    Ok(count) => {
                        if count > 0 {
                            println!(
                                "      {} Harvested {} sessions from {}",
                                "[+]".green(),
                                count.to_string().cyan(),
                                display_name
                            );
                            web_sessions_harvested += count;
                        }
                    }
                    Err(e) => {
                        println!(
                            "      {} Failed to harvest {}: {:?}",
                            "[!]".red(),
                            display_name,
                            e
                        );
                    }
                }
            } else {
                println!("{}", "no session token".yellow());
            }
        } else {
            println!("{}", "not authenticated".yellow());
        }
    }

    if web_sessions_harvested > 0 {
        println!(
            "   {} Total web sessions harvested: {}",
            "[+]".green(),
            web_sessions_harvested.to_string().cyan()
        );
    }

    Ok(())
}

/// Harvest sessions from ChatGPT web interface
fn harvest_chatgpt_sessions(
    conn: &Connection,
    session_token: &str,
    stats: &mut HarvestStats,
) -> Result<usize> {
    use crate::providers::cloud::chatgpt::ChatGPTProvider;
    use crate::providers::cloud::common::{CloudProvider, FetchOptions};

    let provider = ChatGPTProvider::with_session_token(session_token.to_string());

    // List all conversations
    let options = FetchOptions {
        limit: Some(100),
        include_archived: false,
        after: None,
        before: None,
        session_token: Some(session_token.to_string()),
    };

    let conversations = provider
        .list_conversations(&options)
        .context("Failed to list ChatGPT conversations")?;

    let mut harvested = 0;

    for conv_summary in conversations {
        // Check if we already have this conversation
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sessions WHERE id = ? AND provider = 'chatgpt')",
                params![&conv_summary.id],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if exists {
            // Could add update logic here later
            continue;
        }

        // Fetch full conversation
        match provider.fetch_conversation(&conv_summary.id) {
            Ok(conv) => {
                // Insert into database using existing function
                if let Err(e) =
                    insert_cloud_conversation_to_harvest_db(conn, &conv, "chatgpt", None)
                {
                    eprintln!("Failed to insert ChatGPT session: {}", e);
                    continue;
                }
                harvested += 1;
                stats.sessions_added += 1;
            }
            Err(e) => {
                eprintln!("Failed to fetch conversation {}: {}", conv_summary.id, e);
            }
        }

        // Small delay to avoid rate limiting
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    Ok(harvested)
}

/// Harvest sessions from Claude web interface
fn harvest_claude_sessions(
    conn: &Connection,
    session_token: &str,
    stats: &mut HarvestStats,
) -> Result<usize> {
    use crate::providers::cloud::anthropic::AnthropicProvider;
    use crate::providers::cloud::common::{CloudProvider, FetchOptions};

    let provider = AnthropicProvider::with_session_token(session_token.to_string());

    // List all conversations
    let options = FetchOptions {
        limit: Some(100),
        include_archived: false,
        after: None,
        before: None,
        session_token: Some(session_token.to_string()),
    };

    let conversations = provider
        .list_conversations(&options)
        .context("Failed to list Claude conversations")?;

    let mut harvested = 0;

    for conv_summary in conversations {
        // Check if we already have this conversation
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sessions WHERE id = ? AND provider = 'claude')",
                params![&conv_summary.id],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if exists {
            continue;
        }

        // Fetch full conversation
        match provider.fetch_conversation(&conv_summary.id) {
            Ok(conv) => {
                if let Err(e) = insert_cloud_conversation_to_harvest_db(conn, &conv, "claude", None)
                {
                    eprintln!("Failed to insert Claude session: {}", e);
                    continue;
                }
                harvested += 1;
                stats.sessions_added += 1;
            }
            Err(e) => {
                eprintln!("Failed to fetch conversation {}: {}", conv_summary.id, e);
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    Ok(harvested)
}

/// Insert a CloudConversation into the harvest database
fn insert_cloud_conversation_to_harvest_db(
    conn: &Connection,
    conv: &crate::providers::cloud::common::CloudConversation,
    provider: &str,
    workspace_name: Option<&str>,
) -> Result<()> {
    let now = Utc::now().timestamp_millis();
    let created_at = conv.created_at.timestamp_millis();
    let updated_at = conv
        .updated_at
        .map(|dt| dt.timestamp_millis())
        .unwrap_or(created_at);

    // Build a session-like JSON structure for compatibility
    let session_json = serde_json::json!({
        "id": conv.id,
        "title": conv.title,
        "model": conv.model,
        "created_at": conv.created_at.to_rfc3339(),
        "updated_at": conv.updated_at.map(|dt| dt.to_rfc3339()),
        "messages": conv.messages.iter().map(|m| {
            serde_json::json!({
                "id": m.id,
                "role": m.role,
                "content": m.content,
                "timestamp": m.timestamp.map(|dt| dt.to_rfc3339()),
                "model": m.model,
            })
        }).collect::<Vec<_>>(),
    });

    conn.execute(
        r#"
        INSERT OR REPLACE INTO sessions 
        (id, provider, provider_type, workspace_id, workspace_name, title, 
         message_count, created_at, updated_at, harvested_at, session_json)
        VALUES (?, ?, ?, NULL, ?, ?, ?, ?, ?, ?, ?)
        "#,
        params![
            conv.id,
            provider,
            provider,
            workspace_name,
            conv.title.clone().unwrap_or_else(|| "Untitled".to_string()),
            conv.messages.len() as i64,
            created_at,
            updated_at,
            now,
            session_json.to_string(),
        ],
    )?;

    // Also populate messages_v2 table for detailed message storage
    populate_cloud_messages(conn, &conv.id, conv)?;

    Ok(())
}

/// Populate messages_v2 table from a CloudConversation
fn populate_cloud_messages(
    conn: &Connection,
    session_id: &str,
    conv: &crate::providers::cloud::common::CloudConversation,
) -> Result<()> {
    // Delete existing messages for this session to avoid duplicates
    conn.execute("DELETE FROM messages_v2 WHERE session_id = ?", [session_id])?;

    for (idx, message) in conv.messages.iter().enumerate() {
        let timestamp = message.timestamp.map(|dt| dt.timestamp_millis());
        let role = match message.role.as_str() {
            "user" | "human" => "user",
            "assistant" => "assistant",
            "system" => "system",
            other => other,
        };

        conn.execute(
            r#"
            INSERT INTO messages_v2 
            (session_id, message_index, request_id, response_id, role, 
             content_raw, content_markdown, model_id, timestamp, is_canceled, metadata_json)
            VALUES (?, ?, ?, NULL, ?, ?, ?, ?, ?, 0, NULL)
            "#,
            params![
                session_id,
                idx as i64,
                message.id,
                role,
                &message.content,
                &message.content,
                message.model.as_deref(),
                timestamp,
            ],
        )?;
    }

    Ok(())
}

fn update_harvest_metadata(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO harvest_metadata (key, value) VALUES ('last_harvest', datetime('now'))",
        [],
    )?;
    Ok(())
}

fn init_git_tracking(db_path: &Path) -> Result<()> {
    let db_dir = db_path.parent().unwrap_or(Path::new("."));
    let db_name = db_path.file_name().unwrap().to_str().unwrap();

    println!("\n{} Initializing Git Tracking", "[G]".green().bold());

    // Check if already a git repo
    if !db_dir.join(".git").exists() {
        let output = Command::new("git")
            .current_dir(db_dir)
            .args(["init"])
            .output()
            .context("Failed to run git init")?;

        if !output.status.success() {
            anyhow::bail!(
                "Git init failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        println!("{} Initialized git repository", "[+]".green());
    } else {
        println!("{} Git repository already exists", "[i]".blue());
    }

    // Create .gitignore if needed
    let gitignore_path = db_dir.join(".gitignore");
    if !gitignore_path.exists() {
        fs::write(
            &gitignore_path,
            "*.db-journal\n*.db-wal\n*.db-shm\n*.backup\n",
        )?;
        println!("{} Created .gitignore", "[+]".green());
    }

    // Add database to git
    let output = Command::new("git")
        .current_dir(db_dir)
        .args(["add", db_name])
        .output()?;

    if output.status.success() {
        println!("{} Added {} to git", "[+]".green(), db_name);
    }

    // Initial commit if no commits yet
    let output = Command::new("git")
        .current_dir(db_dir)
        .args(["rev-parse", "HEAD"])
        .output()?;

    if !output.status.success() {
        let output = Command::new("git")
            .current_dir(db_dir)
            .args(["commit", "-m", "Initialize harvest database"])
            .output()?;

        if output.status.success() {
            println!("{} Created initial commit", "[+]".green());
        }
    }

    println!("\n{} Git tracking enabled!", "[+]".green().bold());
    println!("   Run 'csm harvest git commit -m \"message\"' to save changes");
    println!("   Run 'csm harvest git log' to view history");

    Ok(())
}

fn git_commit_harvest(db_path: &Path, message: &str) -> Result<()> {
    let db_dir = db_path.parent().unwrap_or(Path::new("."));
    let db_name = db_path.file_name().unwrap().to_str().unwrap();

    // Stage the database
    let output = Command::new("git")
        .current_dir(db_dir)
        .args(["add", db_name])
        .output()
        .context("Failed to stage database")?;

    if !output.status.success() {
        anyhow::bail!(
            "Git add failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Commit
    let output = Command::new("git")
        .current_dir(db_dir)
        .args(["commit", "-m", message])
        .output()
        .context("Failed to commit")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("nothing to commit") {
            println!("{} Nothing to commit", "[i]".blue());
            return Ok(());
        }
        anyhow::bail!("Git commit failed: {}", stderr);
    }

    // Get commit hash
    let output = Command::new("git")
        .current_dir(db_dir)
        .args(["rev-parse", "--short", "HEAD"])
        .output()?;

    let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
    println!("{} Committed: {} - {}", "[+]".green(), hash.cyan(), message);

    Ok(())
}

// ============================================================================
// Share Link Commands
// ============================================================================

/// Import a chat session from a share link URL
pub fn harvest_share(
    db_path: Option<&str>,
    url: &str,
    name: Option<&str>,
    _workspace: Option<&str>,
) -> Result<()> {
    let db_path = get_db_path(db_path)?;

    // Parse the share link
    let share_info = match ShareLinkParser::parse(url) {
        Some(info) => info,
        None => {
            println!("{} Unrecognized share link format", "[!]".yellow());
            println!("   Supported providers: ChatGPT, Claude, Gemini, Perplexity");
            println!("   Example URLs:");
            println!("   - https://chat.openai.com/share/abc123...");
            println!("   - https://claude.ai/share/xyz789...");
            anyhow::bail!("Could not parse share link URL");
        }
    };

    println!("{}", "=".repeat(60).cyan());
    println!("{}", " Share Link Import ".bold().cyan());
    println!("{}", "=".repeat(60).cyan());
    println!();

    println!(
        "{} Detected provider: {}",
        "[i]".blue(),
        share_info.provider.bold()
    );
    println!("{} Share ID: {}", "[i]".blue(), share_info.share_id);

    // Open or create the database with new schema
    let db = ChatDatabase::open(&db_path)?;

    // Check if already imported
    let conn = db.connection();
    let existing: Option<(String, i64)> = conn
        .query_row(
            "SELECT id, imported FROM share_links WHERE url = ?",
            [url],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
        .ok();

    if let Some((id, imported)) = existing {
        println!();
        println!(
            "{} This share link has already been registered (ID: {})",
            "[!]".yellow(),
            id
        );

        let status = if imported == 1 { "imported" } else { "pending" };
        println!("{} Current status: {}", "[i]".blue(), status);

        if imported == 1 {
            println!(
                "{} Session already imported - no action needed",
                "[+]".green()
            );
            return Ok(());
        }
    } else {
        // Insert the share link as pending
        let link_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO share_links (id, url, provider, share_id, title, imported, created_at) VALUES (?, ?, ?, ?, ?, 0, ?)",
            params![link_id, url, share_info.provider, share_info.share_id, name, now],
        )?;

        println!(
            "{} Registered share link (ID: {})",
            "[+]".green(),
            &link_id[..8]
        );
    }

    println!();
    println!("{}", "-".repeat(60).dimmed());
    println!();

    // TODO: Implement actual content fetching
    // For now, we just register the share link and mark it as pending
    // Actual fetching would require:
    // 1. Browser cookie extraction (already implemented in browser.rs)
    // 2. HTTP request with auth cookies
    // 3. HTML/JSON parsing of the share page
    // 4. Conversion to our universal format

    println!("{} Share link registered as pending", "[i]".blue());
    println!();
    println!("{}", "Note:".bold().yellow());
    println!("   Automatic content fetching from share links is not yet implemented.");
    println!("   For now, you can:");
    println!("   1. Open the share link in your browser");
    println!("   2. Export the conversation manually");
    println!("   3. Import with: csm import <file>");
    println!();
    println!("   Or use 'csm harvest shares' to view pending links.");

    Ok(())
}

/// List share links in the harvest database
pub fn harvest_shares(
    db_path: Option<&str>,
    status_filter: Option<&str>,
    limit: usize,
) -> Result<()> {
    let db_path = get_db_path(db_path)?;

    if !db_path.exists() {
        anyhow::bail!("Harvest database not found. Run 'csm harvest init' first.");
    }

    let db = ChatDatabase::open(&db_path)?;
    let conn = db.connection();

    println!("{}", "=".repeat(70).cyan());
    println!("{}", " Share Links ".bold().cyan());
    println!("{}", "=".repeat(70).cyan());
    println!();

    let query = match status_filter {
        Some("pending") => format!(
            "SELECT id, url, provider, share_id, title, imported, created_at 
             FROM share_links WHERE imported = 0 ORDER BY created_at DESC LIMIT {}",
            limit
        ),
        Some("imported") => format!(
            "SELECT id, url, provider, share_id, title, imported, created_at 
             FROM share_links WHERE imported = 1 ORDER BY created_at DESC LIMIT {}",
            limit
        ),
        Some(_) | None => format!(
            "SELECT id, url, provider, share_id, title, imported, created_at 
             FROM share_links ORDER BY created_at DESC LIMIT {}",
            limit
        ),
    };

    let mut stmt = conn.prepare(&query)?;

    let rows: Vec<_> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    if rows.is_empty() {
        if let Some(status) = status_filter {
            println!("{} No share links with status '{}'", "[i]".blue(), status);
        } else {
            println!("{} No share links found", "[i]".blue());
        }
        println!("   Use 'csm harvest share <url>' to add a share link");
        return Ok(());
    }

    // Count by status
    let pending_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM share_links WHERE imported = 0",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let imported_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM share_links WHERE imported = 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    println!(
        "{} {} pending, {} imported",
        "[i]".blue(),
        pending_count.to_string().yellow(),
        imported_count.to_string().green()
    );
    println!();

    for (id, url, provider, _share_id, title, imported, created_at) in rows {
        let status = if imported == 1 { "imported" } else { "pending" };
        let status_colored = if imported == 1 {
            status.green()
        } else {
            status.yellow()
        };

        println!(
            "{} [{}] {} - {}",
            format!("#{}", &id[..8]).dimmed(),
            status_colored,
            provider.bold(),
            title.as_deref().unwrap_or("(untitled)")
        );

        // Truncate URL for display
        let display_url = if url.len() > 60 {
            format!("{}...", &url[..57])
        } else {
            url.clone()
        };
        println!("   {} {}", "URL:".dimmed(), display_url.dimmed());

        // Format timestamp
        let timestamp = chrono::DateTime::from_timestamp(created_at, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| created_at.to_string());
        println!("   {} {}", "Added:".dimmed(), timestamp.dimmed());
        println!();
    }

    Ok(())
}

// ============================================================================
// Checkpoint Commands
// ============================================================================

/// Create a checkpoint (version snapshot) of a session
pub fn harvest_checkpoint(
    db_path: Option<&str>,
    session_id: &str,
    message: Option<&str>,
) -> Result<()> {
    let db_path = get_db_path(db_path)?;

    if !db_path.exists() {
        anyhow::bail!("Harvest database not found. Run 'csm harvest init' first.");
    }

    let db = ChatDatabase::open(&db_path)?;
    let conn = db.connection();

    // Find the session
    let session: Option<(i64, String, i64)> = conn
        .query_row(
            "SELECT id, session_id, message_count FROM sessions WHERE session_id = ? OR id = ?",
            params![session_id, session_id.parse::<i64>().unwrap_or(-1)],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .ok();

    let (internal_id, actual_session_id, msg_count) = match session {
        Some(s) => s,
        None => {
            println!("{} Session not found: {}", "[!]".red(), session_id);
            println!("   Use 'csm harvest list' to see available sessions");
            anyhow::bail!("Session not found");
        }
    };

    // Get current checkpoint number
    let checkpoint_num: i64 = conn.query_row(
        "SELECT COALESCE(MAX(checkpoint_number), 0) + 1 FROM checkpoints WHERE session_id = ?",
        [internal_id],
        |row| row.get(0),
    )?;

    // Calculate content hash (simple hash of message count + session_id for now)
    let content_hash = format!(
        "{:x}",
        md5_hash(&format!("{}:{}", actual_session_id, msg_count))
    );

    // Get message snapshot
    let messages: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT content FROM messages WHERE session_id = ? ORDER BY message_index")?;
        let rows = stmt.query_map([internal_id], |row| row.get::<_, String>(0))?;
        rows.filter_map(|r| r.ok()).collect()
    };

    let snapshot = serde_json::json!({
        "message_count": msg_count,
        "messages": messages,
    });

    let default_message = format!("Checkpoint {}", checkpoint_num);
    let message_text = message.unwrap_or(&default_message);

    // Create checkpoint
    conn.execute(
        "INSERT INTO checkpoints (session_id, checkpoint_number, message, message_count, content_hash, snapshot)
         VALUES (?, ?, ?, ?, ?, ?)",
        params![
            internal_id,
            checkpoint_num,
            message_text,
            msg_count,
            content_hash,
            snapshot.to_string()
        ],
    )?;

    println!("{}", "=".repeat(60).cyan());
    println!("{}", " Checkpoint Created ".bold().cyan());
    println!("{}", "=".repeat(60).cyan());
    println!();
    println!("{} Session: {}", "[+]".green(), actual_session_id);
    println!(
        "{} Checkpoint #{}: {}",
        "[+]".green(),
        checkpoint_num,
        message_text
    );
    println!("{} Messages: {}", "[i]".blue(), msg_count);
    println!("{} Hash: {}", "[i]".blue(), &content_hash[..16]);
    println!();
    println!(
        "   Use 'csm harvest checkpoints {}' to view history",
        session_id
    );

    Ok(())
}

/// List checkpoints for a session
pub fn harvest_checkpoints(db_path: Option<&str>, session_id: &str) -> Result<()> {
    let db_path = get_db_path(db_path)?;

    if !db_path.exists() {
        anyhow::bail!("Harvest database not found. Run 'csm harvest init' first.");
    }

    let db = ChatDatabase::open(&db_path)?;
    let conn = db.connection();

    // Find the session
    let session: Option<(i64, String, String)> = conn
        .query_row(
            "SELECT id, session_id, name FROM sessions WHERE session_id = ? OR id = ?",
            params![session_id, session_id.parse::<i64>().unwrap_or(-1)],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                ))
            },
        )
        .ok();

    let (internal_id, actual_session_id, name) = match session {
        Some(s) => s,
        None => {
            println!("{} Session not found: {}", "[!]".red(), session_id);
            anyhow::bail!("Session not found");
        }
    };

    println!("{}", "=".repeat(70).cyan());
    println!("{}", " Session Checkpoints ".bold().cyan());
    println!("{}", "=".repeat(70).cyan());
    println!();
    println!(
        "{} Session: {} {}",
        "[i]".blue(),
        actual_session_id,
        if !name.is_empty() {
            format!("({})", name)
        } else {
            String::new()
        }
    );
    println!();

    let mut stmt = conn.prepare(
        "SELECT checkpoint_number, message, message_count, content_hash, created_at
         FROM checkpoints WHERE session_id = ? ORDER BY checkpoint_number DESC",
    )?;

    let checkpoints: Vec<_> = stmt
        .query_map([internal_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    if checkpoints.is_empty() {
        println!("{} No checkpoints found for this session", "[i]".blue());
        println!(
            "   Use 'csm harvest checkpoint {} -m \"message\"' to create one",
            session_id
        );
        return Ok(());
    }

    println!("{} {} checkpoints found:", "[i]".blue(), checkpoints.len());
    println!();

    for (num, msg, msg_count, hash, created_at) in checkpoints {
        println!("  {} #{} - {}", "*".cyan(), num.to_string().bold(), msg);
        println!(
            "       {} messages | {} | {}",
            msg_count,
            &hash[..12],
            created_at.dimmed()
        );
    }

    println!();
    println!(
        "   Use 'csm harvest restore {} <checkpoint>' to restore",
        session_id
    );

    Ok(())
}

/// Restore a session to a previous checkpoint
pub fn harvest_restore_checkpoint(
    db_path: Option<&str>,
    session_id: &str,
    checkpoint_number: i64,
) -> Result<()> {
    let db_path = get_db_path(db_path)?;

    if !db_path.exists() {
        anyhow::bail!("Harvest database not found. Run 'csm harvest init' first.");
    }

    let db = ChatDatabase::open(&db_path)?;
    let conn = db.connection();

    // Find the session
    let session: Option<(i64, String)> = conn
        .query_row(
            "SELECT id, session_id FROM sessions WHERE session_id = ? OR id = ?",
            params![session_id, session_id.parse::<i64>().unwrap_or(-1)],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok();

    let (internal_id, actual_session_id) = match session {
        Some(s) => s,
        None => {
            println!("{} Session not found: {}", "[!]".red(), session_id);
            anyhow::bail!("Session not found");
        }
    };

    // Find the checkpoint
    let checkpoint: Option<(String, i64)> = conn
        .query_row(
            "SELECT snapshot, message_count FROM checkpoints 
         WHERE session_id = ? AND checkpoint_number = ?",
            params![internal_id, checkpoint_number],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
        .ok();

    let (snapshot_json, original_msg_count) = match checkpoint {
        Some(c) => c,
        None => {
            println!(
                "{} Checkpoint #{} not found for session {}",
                "[!]".red(),
                checkpoint_number,
                session_id
            );
            println!(
                "   Use 'csm harvest checkpoints {}' to see available checkpoints",
                session_id
            );
            anyhow::bail!("Checkpoint not found");
        }
    };

    // Parse snapshot
    let snapshot: serde_json::Value =
        serde_json::from_str(&snapshot_json).context("Failed to parse checkpoint snapshot")?;

    let messages = snapshot["messages"]
        .as_array()
        .context("Invalid snapshot format")?;

    println!("{}", "=".repeat(60).cyan());
    println!("{}", " Restore Checkpoint ".bold().yellow());
    println!("{}", "=".repeat(60).cyan());
    println!();
    println!("{} Session: {}", "[i]".blue(), actual_session_id);
    println!(
        "{} Restoring to checkpoint #{}",
        "[!]".yellow(),
        checkpoint_number
    );
    println!(
        "{} Messages to restore: {}",
        "[i]".blue(),
        original_msg_count
    );
    println!();

    // Delete current messages
    let deleted = conn.execute("DELETE FROM messages WHERE session_id = ?", [internal_id])?;

    println!("{} Removed {} current messages", "[-]".red(), deleted);

    // Restore messages from snapshot
    for (idx, msg) in messages.iter().enumerate() {
        if let Some(content) = msg.as_str() {
            conn.execute(
                "INSERT INTO messages (session_id, message_index, role, content) VALUES (?, ?, 'unknown', ?)",
                params![internal_id, idx as i64, content],
            )?;
        }
    }

    // Update session message count
    conn.execute(
        "UPDATE sessions SET message_count = ?, updated_at = datetime('now') WHERE id = ?",
        params![original_msg_count, internal_id],
    )?;

    println!(
        "{} Restored {} messages from checkpoint",
        "[+]".green(),
        messages.len()
    );
    println!();
    println!(
        "{} Session restored to checkpoint #{}",
        "[+]".green().bold(),
        checkpoint_number
    );

    Ok(())
}

// ============================================================================
// Search Commands
// ============================================================================

/// Rebuild the FTS index from messages_v2 table
pub fn harvest_rebuild_fts(db_path: Option<&str>) -> Result<()> {
    let db_path = get_db_path(db_path)?;

    if !db_path.exists() {
        anyhow::bail!("Harvest database not found. Run 'csm harvest init' first.");
    }

    let conn = Connection::open(&db_path)?;

    println!("{}", "=".repeat(70).cyan());
    println!("{} Rebuilding Full-Text Search Index", "[*]".bold());
    println!("{}", "=".repeat(70).cyan());
    println!();

    // Drop existing FTS table and triggers
    println!("{} Dropping old FTS index...", "[*]".blue());
    conn.execute_batch(
        r#"
        DROP TRIGGER IF EXISTS messages_v2_ai;
        DROP TRIGGER IF EXISTS messages_v2_ad;
        DROP TRIGGER IF EXISTS messages_v2_au;
        DROP TABLE IF EXISTS messages_fts;
        "#,
    )?;

    // Create new FTS table with triggers
    println!("{} Creating new FTS index...", "[*]".blue());
    conn.execute_batch(
        r#"
        CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
            content_raw
        );
        
        CREATE TRIGGER IF NOT EXISTS messages_v2_ai AFTER INSERT ON messages_v2 BEGIN
            INSERT INTO messages_fts(rowid, content_raw) VALUES (new.id, new.content_raw);
        END;
        
        CREATE TRIGGER IF NOT EXISTS messages_v2_ad AFTER DELETE ON messages_v2 BEGIN
            INSERT INTO messages_fts(messages_fts, rowid, content_raw) VALUES ('delete', old.id, old.content_raw);
        END;
        
        CREATE TRIGGER IF NOT EXISTS messages_v2_au AFTER UPDATE ON messages_v2 BEGIN
            INSERT INTO messages_fts(messages_fts, rowid, content_raw) VALUES ('delete', old.id, old.content_raw);
            INSERT INTO messages_fts(rowid, content_raw) VALUES (new.id, new.content_raw);
        END;
        "#,
    )?;

    // Count total messages
    let total_messages: i64 =
        conn.query_row("SELECT COUNT(*) FROM messages_v2", [], |row| row.get(0))?;

    // Populate FTS from existing messages
    println!("{} Indexing {} messages...", "[*]".blue(), total_messages);

    conn.execute(
        "INSERT INTO messages_fts(rowid, content_raw) SELECT id, content_raw FROM messages_v2",
        [],
    )?;

    // Verify the index
    let indexed: i64 = conn.query_row("SELECT COUNT(*) FROM messages_fts", [], |row| row.get(0))?;

    println!();
    println!("{} FTS index rebuilt successfully!", "[+]".green().bold());
    println!("   {} messages indexed", indexed);

    Ok(())
}

/// Full-text search across all sessions
pub fn harvest_search(
    db_path: Option<&str>,
    query: &str,
    provider_filter: Option<&str>,
    limit: usize,
) -> Result<()> {
    let db_path = get_db_path(db_path)?;

    if !db_path.exists() {
        anyhow::bail!("Harvest database not found. Run 'csm harvest init' first.");
    }

    let db = ChatDatabase::open(&db_path)?;
    let conn = db.connection();

    println!("{}", "=".repeat(70).cyan());
    println!("{} Search: {}", "[?]".bold(), query.bold());
    println!("{}", "=".repeat(70).cyan());
    println!();

    // Results: (session_id, provider, title, content)
    let results: Vec<(String, String, String, String)> = {
        // Check if FTS table exists
        let fts_exists: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='messages_fts'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if fts_exists {
            // Use FTS search - query the FTS table and join with messages/sessions
            let sql = format!(
                "SELECT s.id, s.provider, s.title, m.content_raw
                 FROM messages_fts fts
                 JOIN messages_v2 m ON m.id = fts.rowid
                 JOIN sessions s ON m.session_id = s.id
                 WHERE messages_fts MATCH ?
                 {}
                 LIMIT {}",
                if provider_filter.is_some() {
                    "AND s.provider = ?"
                } else {
                    ""
                },
                limit
            );

            let mut stmt = conn.prepare(&sql)?;

            if let Some(provider) = provider_filter {
                stmt.query_map(params![query, provider], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                        row.get::<_, String>(3)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?
            } else {
                stmt.query_map([query], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                        row.get::<_, String>(3)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?
            }
        } else {
            // Fall back to LIKE search
            let search_pattern = format!("%{}%", query);
            let sql = format!(
                "SELECT s.id, s.provider, s.title, m.content_raw
                 FROM messages_v2 m
                 JOIN sessions s ON m.session_id = s.id
                 WHERE m.content_raw LIKE ?
                 {}
                 LIMIT {}",
                if provider_filter.is_some() {
                    "AND s.provider = ?"
                } else {
                    ""
                },
                limit
            );

            let mut stmt = conn.prepare(&sql)?;

            if let Some(provider) = provider_filter {
                stmt.query_map(params![search_pattern, provider], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                        row.get::<_, String>(3)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?
            } else {
                stmt.query_map([search_pattern], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                        row.get::<_, String>(3)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?
            }
        }
    };

    if results.is_empty() {
        println!("{} No results found for '{}'", "[i]".blue(), query);
        return Ok(());
    }

    println!("{} Found {} result(s):", "[i]".blue(), results.len());
    println!();

    for (session_id, provider, title, content) in results {
        // Highlight the search term in content
        let display_name = if title.is_empty() {
            session_id.clone()
        } else {
            format!("{} ({})", title, &session_id[..8.min(session_id.len())])
        };

        println!(
            "{} {} [{}]",
            "*".cyan(),
            display_name.bold(),
            provider.dimmed()
        );

        // Show snippet around the match
        let snippet = create_search_snippet(&content, query, 100);
        println!("   {}", snippet.dimmed());
        println!();
    }

    Ok(())
}

/// Create a search result snippet with the query highlighted
fn create_search_snippet(content: &str, query: &str, max_len: usize) -> String {
    let content_lower = content.to_lowercase();
    let query_lower = query.to_lowercase();

    if let Some(pos) = content_lower.find(&query_lower) {
        let start = pos.saturating_sub(max_len / 2);
        let end = (pos + query.len() + max_len / 2).min(content.len());

        let mut snippet = String::new();
        if start > 0 {
            snippet.push_str("...");
        }
        snippet.push_str(&content[start..end]);
        if end < content.len() {
            snippet.push_str("...");
        }

        // Replace newlines with spaces for display
        snippet.replace('\n', " ").replace('\r', "")
    } else {
        // Fallback: just show first max_len chars
        if content.len() > max_len {
            format!("{}...", &content[..max_len])
        } else {
            content.to_string()
        }
    }
}

/// Simple MD5 hash for content checksums
fn md5_hash(data: &str) -> u128 {
    // Simple hash implementation (not cryptographically secure, just for checksums)
    let mut hash: u128 = 0;
    for (i, byte) in data.bytes().enumerate() {
        hash = hash.wrapping_add((byte as u128).wrapping_mul((i as u128).wrapping_add(1)));
        hash = hash.rotate_left(7);
    }
    hash
}
