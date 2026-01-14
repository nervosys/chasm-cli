// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Auto-detection commands for workspaces and providers

use anyhow::Result;
use colored::*;

use crate::models::Workspace;
use crate::providers::{ProviderRegistry, ProviderType};
use crate::workspace::{
    discover_workspaces, find_workspace_by_path, get_chat_sessions_from_workspace,
};

/// Detect workspace information for a given path
pub fn detect_workspace(path: Option<&str>) -> Result<()> {
    let project_path = path.map(|p| p.to_string()).unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string())
    });

    println!("\n{} Detecting Workspace", "[D]".blue().bold());
    println!("{}", "=".repeat(60));
    println!("{} Path: {}", "[*]".blue(), project_path.cyan());

    match find_workspace_by_path(&project_path)? {
        Some((ws_id, ws_dir, ws_name)) => {
            println!("\n{} Workspace Found!", "[+]".green().bold());
            println!("   {} ID: {}", "[*]".blue(), &ws_id[..16.min(ws_id.len())]);
            println!("   {} Directory: {}", "[*]".blue(), ws_dir.display());
            if let Some(name) = ws_name {
                println!("   {} Name: {}", "[*]".blue(), name.cyan());
            }

            // Get session count
            if let Ok(sessions) = get_chat_sessions_from_workspace(&ws_dir) {
                println!("   {} Sessions: {}", "[*]".blue(), sessions.len());

                if !sessions.is_empty() {
                    let total_messages: usize =
                        sessions.iter().map(|s| s.session.request_count()).sum();
                    println!("   {} Total Messages: {}", "[*]".blue(), total_messages);
                }
            }

            // Detect provider
            println!("\n{} Provider Detection:", "[*]".blue());
            println!(
                "   {} Provider: {}",
                "[*]".blue(),
                "GitHub Copilot (VS Code)".cyan()
            );
        }
        None => {
            println!("\n{} No workspace found for this path", "[X]".red());
            println!(
                "{} The project may not have been opened in VS Code yet",
                "[i]".yellow()
            );

            // Check if there are similar workspaces
            let all_workspaces = discover_workspaces()?;
            let path_lower = project_path.to_lowercase();
            let similar: Vec<&Workspace> = all_workspaces
                .iter()
                .filter(|ws| {
                    ws.project_path
                        .as_ref()
                        .map(|p| {
                            p.to_lowercase().contains(&path_lower)
                                || path_lower.contains(&p.to_lowercase())
                        })
                        .unwrap_or(false)
                })
                .take(5)
                .collect();

            if !similar.is_empty() {
                println!("\n{} Similar workspaces found:", "[i]".cyan());
                for ws in similar {
                    if let Some(p) = &ws.project_path {
                        println!("   {} {}...", p.cyan(), &ws.hash[..8.min(ws.hash.len())]);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Detect available providers and their status
pub fn detect_providers(with_sessions: bool) -> Result<()> {
    println!("\n{} Detecting Providers", "[D]".blue().bold());
    println!("{}", "=".repeat(60));

    let registry = ProviderRegistry::new();
    let mut found_count = 0;
    let mut with_sessions_count = 0;

    let all_provider_types = vec![
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

    for provider_type in all_provider_types {
        if let Some(provider) = registry.get_provider(provider_type) {
            let available = provider.is_available();
            let session_count = if available {
                provider.list_sessions().map(|s| s.len()).unwrap_or(0)
            } else {
                0
            };

            if with_sessions && session_count == 0 {
                continue;
            }

            found_count += 1;
            if session_count > 0 {
                with_sessions_count += 1;
            }

            let status = if available {
                if session_count > 0 {
                    format!(
                        "{} ({} sessions)",
                        "+".green(),
                        session_count.to_string().cyan()
                    )
                } else {
                    format!("{} (no sessions)", "+".green())
                }
            } else {
                format!("{} not available", "x".red())
            };

            println!("   {} {}: {}", "[*]".blue(), provider.name().bold(), status);

            // Show endpoint for API-based providers
            if available {
                if let Some(endpoint) = provider_type.default_endpoint() {
                    println!("      {} Endpoint: {}", "`".dimmed(), endpoint.dimmed());
                }
                if let Some(path) = provider.sessions_path() {
                    println!(
                        "      {} Path: {}",
                        "`".dimmed(),
                        path.display().to_string().dimmed()
                    );
                }
            }
        }
    }

    println!("\n{} Summary:", "[*]".green().bold());
    println!("   {} providers available", found_count.to_string().cyan());
    println!(
        "   {} providers with sessions",
        with_sessions_count.to_string().cyan()
    );

    Ok(())
}

/// Detect which provider a session belongs to
pub fn detect_session(session_id: &str, path: Option<&str>) -> Result<()> {
    println!("\n{} Detecting Session Provider", "[D]".blue().bold());
    println!("{}", "=".repeat(60));
    println!("{} Session: {}", "[*]".blue(), session_id.cyan());

    let registry = ProviderRegistry::new();
    let session_lower = session_id.to_lowercase();
    let mut found = false;

    // First check VS Code workspaces
    let project_path = path.map(|p| p.to_string()).unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string())
    });

    // Check in VS Code/Copilot workspaces
    if let Ok(Some((_ws_id, ws_dir, ws_name))) = find_workspace_by_path(&project_path) {
        if let Ok(sessions) = get_chat_sessions_from_workspace(&ws_dir) {
            for swp in &sessions {
                let sid = swp
                    .session
                    .session_id
                    .as_ref()
                    .map(|s| s.to_lowercase())
                    .unwrap_or_default();
                let title = swp.session.title().to_lowercase();
                let filename = swp
                    .path
                    .file_name()
                    .map(|f| f.to_string_lossy().to_lowercase())
                    .unwrap_or_default();

                if sid.contains(&session_lower)
                    || title.contains(&session_lower)
                    || filename.contains(&session_lower)
                {
                    found = true;
                    println!("\n{} Session Found!", "[+]".green().bold());
                    println!("   {} Provider: {}", "[*]".blue(), "GitHub Copilot".cyan());
                    println!("   {} Title: {}", "[*]".blue(), swp.session.title());
                    println!("   {} File: {}", "[*]".blue(), swp.path.display());
                    println!(
                        "   {} Messages: {}",
                        "[*]".blue(),
                        swp.session.request_count()
                    );
                    if let Some(name) = &ws_name {
                        println!("   {} Workspace: {}", "[*]".blue(), name);
                    }
                    break;
                }
            }
        }
    }

    // Check other providers
    if !found {
        let provider_types = vec![
            ProviderType::Cursor,
            ProviderType::Ollama,
            ProviderType::Jan,
            ProviderType::Gpt4All,
            ProviderType::LmStudio,
        ];

        for provider_type in provider_types {
            if let Some(provider) = registry.get_provider(provider_type) {
                if provider.is_available() {
                    if let Ok(sessions) = provider.list_sessions() {
                        for session in sessions {
                            let sid = session
                                .session_id
                                .as_ref()
                                .map(|s| s.to_lowercase())
                                .unwrap_or_default();
                            let title = session.title().to_lowercase();

                            if sid.contains(&session_lower) || title.contains(&session_lower) {
                                found = true;
                                println!("\n{} Session Found!", "[+]".green().bold());
                                println!(
                                    "   {} Provider: {}",
                                    "[*]".blue(),
                                    provider.name().cyan()
                                );
                                println!("   {} Title: {}", "[*]".blue(), session.title());
                                println!(
                                    "   {} Messages: {}",
                                    "[*]".blue(),
                                    session.request_count()
                                );
                                break;
                            }
                        }
                    }
                }
            }
            if found {
                break;
            }
        }
    }

    if !found {
        println!("\n{} Session not found", "[X]".red());
        println!(
            "{} Try providing a more specific session ID or check the path",
            "[i]".yellow()
        );
    }

    Ok(())
}

/// Detect everything (workspace, providers, sessions) for a path
pub fn detect_all(path: Option<&str>, verbose: bool) -> Result<()> {
    let project_path = path.map(|p| p.to_string()).unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string())
    });

    println!("\n{} Auto-Detection Report", "[D]".blue().bold());
    println!("{}", "=".repeat(70));
    println!("{} Path: {}", "[*]".blue(), project_path.cyan());
    println!();

    // 1. Workspace Detection
    println!("{} Workspace", "---".dimmed());
    let workspace_info = find_workspace_by_path(&project_path)?;

    match &workspace_info {
        Some((ws_id, ws_dir, ws_name)) => {
            println!("   {} Status: {}", "[+]".green(), "Found".green());
            println!(
                "   {} ID: {}...",
                "[*]".blue(),
                &ws_id[..16.min(ws_id.len())]
            );
            if let Some(name) = ws_name {
                println!("   {} Name: {}", "[*]".blue(), name.cyan());
            }

            // Get sessions from workspace
            if let Ok(sessions) = get_chat_sessions_from_workspace(ws_dir) {
                println!("   {} Sessions: {}", "[*]".blue(), sessions.len());

                if verbose && !sessions.is_empty() {
                    println!("\n   {} Recent Sessions:", "[*]".blue());
                    for (i, swp) in sessions.iter().take(5).enumerate() {
                        println!(
                            "      {}. {} ({} messages)",
                            i + 1,
                            truncate(&swp.session.title(), 40),
                            swp.session.request_count()
                        );
                    }
                    if sessions.len() > 5 {
                        println!("      ... and {} more", sessions.len() - 5);
                    }
                }
            }
        }
        None => {
            println!("   {} Status: {}", "[X]".red(), "Not found".red());
            println!(
                "   {} Open this project in VS Code to create a workspace",
                "[i]".yellow()
            );
        }
    }
    println!();

    // 2. Provider Detection
    println!("{} Available Providers", "---".dimmed());

    let registry = ProviderRegistry::new();
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

    let mut total_sessions = 0;
    let mut provider_summary: Vec<(String, usize)> = Vec::new();

    for provider_type in provider_types {
        if let Some(provider) = registry.get_provider(provider_type) {
            if provider.is_available() {
                let session_count = provider.list_sessions().map(|s| s.len()).unwrap_or(0);

                if session_count > 0 || verbose {
                    let status = if session_count > 0 {
                        format!("{} sessions", session_count.to_string().cyan())
                    } else {
                        "ready".dimmed().to_string()
                    };
                    println!("   {} {}: {}", "[+]".green(), provider.name(), status);

                    total_sessions += session_count;
                    if session_count > 0 {
                        provider_summary.push((provider.name().to_string(), session_count));
                    }
                }
            }
        }
    }

    if provider_summary.is_empty() && !verbose {
        println!("   {} No providers with sessions found", "[i]".yellow());
        println!(
            "   {} Use --verbose to see all available providers",
            "[i]".dimmed()
        );
    }
    println!();

    // 3. Summary
    println!("{} Summary", "---".dimmed());

    let ws_status = if workspace_info.is_some() {
        "Yes".green()
    } else {
        "No".red()
    };
    println!("   {} Workspace detected: {}", "[*]".blue(), ws_status);
    println!(
        "   {} Total providers with sessions: {}",
        "[*]".blue(),
        provider_summary.len()
    );
    println!(
        "   {} Total sessions across providers: {}",
        "[*]".blue(),
        total_sessions
    );

    // 4. Recommendations
    if workspace_info.is_none() || total_sessions == 0 {
        println!();
        println!("{} Recommendations", "---".dimmed());

        if workspace_info.is_none() {
            println!(
                "   {} Open this project in VS Code to enable chat history tracking",
                "[->]".cyan()
            );
        }

        if total_sessions == 0 {
            println!(
                "   {} Start a chat session in your IDE to create history",
                "[->]".cyan()
            );
        }
    }

    Ok(())
}

/// Helper function to truncate strings
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// List available models from all providers
pub fn list_models(provider_filter: Option<&str>) -> Result<()> {
    use tabled::{settings::Style, Table, Tabled};

    println!("\n{} Listing Available Models", "[M]".blue().bold());
    println!("{}", "=".repeat(60));

    #[derive(Tabled)]
    struct ModelRow {
        #[tabled(rename = "Provider")]
        provider: String,
        #[tabled(rename = "Model")]
        model: String,
        #[tabled(rename = "Status")]
        status: String,
    }

    let registry = ProviderRegistry::new();
    let mut rows: Vec<ModelRow> = Vec::new();

    let all_provider_types = vec![
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

    for provider_type in all_provider_types {
        let provider_name = format!("{:?}", provider_type).to_lowercase();

        // Filter by provider if specified
        if let Some(filter) = provider_filter {
            if !provider_name.contains(&filter.to_lowercase()) {
                continue;
            }
        }

        if let Some(provider) = registry.get_provider(provider_type) {
            if provider.is_available() {
                match provider.list_models() {
                    Ok(models) if !models.is_empty() => {
                        for model in models {
                            rows.push(ModelRow {
                                provider: provider_name.clone(),
                                model: model.clone(),
                                status: "available".to_string(),
                            });
                        }
                    }
                    Ok(_) => {
                        // Provider available but no models listed
                        rows.push(ModelRow {
                            provider: provider_name.clone(),
                            model: "(query endpoint)".to_string(),
                            status: "online".to_string(),
                        });
                    }
                    Err(_) => {
                        rows.push(ModelRow {
                            provider: provider_name.clone(),
                            model: "(error)".to_string(),
                            status: "error".to_string(),
                        });
                    }
                }
            } else if provider_filter.is_some() {
                // Only show offline providers if specifically filtered
                rows.push(ModelRow {
                    provider: provider_name.clone(),
                    model: "(not running)".to_string(),
                    status: "offline".to_string(),
                });
            }
        }
    }

    if rows.is_empty() {
        println!("{} No models found", "[!]".yellow());
        println!(
            "   Start a local LLM provider (Ollama, LM Studio, etc.) to see available models."
        );
        return Ok(());
    }

    let unique_providers: std::collections::HashSet<_> =
        rows.iter().map(|r| r.provider.clone()).collect();
    let provider_count = unique_providers.len();
    let row_count = rows.len();

    let table = Table::new(&rows).with(Style::ascii_rounded()).to_string();

    // Apply colors to table output (after width calculation)
    for line in table.lines() {
        let colored = line
            .replace("| ollama ", &format!("| {} ", "ollama".cyan()))
            .replace("| vllm ", &format!("| {} ", "vllm".cyan()))
            .replace("| foundry ", &format!("| {} ", "foundry".cyan()))
            .replace("| lmstudio ", &format!("| {} ", "lmstudio".cyan()))
            .replace("| localai ", &format!("| {} ", "localai".cyan()))
            .replace("| textgenwebui ", &format!("| {} ", "textgenwebui".cyan()))
            .replace("| jan ", &format!("| {} ", "jan".cyan()))
            .replace("| gpt4all ", &format!("| {} ", "gpt4all".cyan()))
            .replace("| llamafile ", &format!("| {} ", "llamafile".cyan()))
            .replace("| available ", &format!("| {} ", "available".green()))
            .replace("| online ", &format!("| {} ", "online".green()))
            .replace("| error ", &format!("| {} ", "error".red()))
            .replace("| offline ", &format!("| {} ", "offline".dimmed()));
        println!("{}", colored);
    }

    println!(
        "\n{} Found {} model(s) from {} provider(s)",
        "[=]".blue(),
        row_count.to_string().yellow(),
        provider_count.to_string().yellow()
    );

    Ok(())
}
