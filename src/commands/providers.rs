// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Provider management commands

use anyhow::Result;
use colored::*;

use crate::providers::{
    config::{CsmConfig, ProviderConfig},
    discovery::print_provider_summary,
    ProviderRegistry, ProviderType,
};

/// List all discovered providers
pub fn list_providers() -> Result<()> {
    let registry = ProviderRegistry::new();
    print_provider_summary(&registry);
    Ok(())
}

/// Show detailed info about a specific provider
pub fn provider_info(provider_name: &str) -> Result<()> {
    let provider_type = parse_provider_name(provider_name)?;
    let registry = ProviderRegistry::new();

    if let Some(provider) = registry.get_provider(provider_type) {
        println!("{}", format!("Provider: {}", provider.name()).bold());
        println!();

        println!("  Type:      {}", provider_type.display_name());
        println!(
            "  Available: {}",
            if provider.is_available() {
                "Yes".green()
            } else {
                "No".red()
            }
        );

        if let Some(path) = provider.sessions_path() {
            println!("  Data Path: {}", path.display());
        }

        if let Some(endpoint) = provider_type.default_endpoint() {
            println!("  Endpoint:  {}", endpoint);
        }

        println!(
            "  OpenAI Compatible: {}",
            if provider_type.is_openai_compatible() {
                "Yes".green()
            } else {
                "No".dimmed()
            }
        );

        println!(
            "  File Storage: {}",
            if provider_type.uses_file_storage() {
                "Yes".green()
            } else {
                "No".dimmed()
            }
        );

        // Show sessions if available
        if provider.is_available() {
            match provider.list_sessions() {
                Ok(sessions) => {
                    println!();
                    println!("  Sessions:  {}", sessions.len());

                    if !sessions.is_empty() {
                        println!();
                        println!("  Recent sessions:");
                        for session in sessions.iter().take(5) {
                            println!("    - {}", session.title());
                        }
                    }
                }
                Err(_) => {
                    println!("  Sessions:  (unable to list)");
                }
            }
        }
    } else {
        eprintln!("{} Provider not found: {}", "Error:".red(), provider_name);
        eprintln!();
        eprintln!("Available providers:");
        list_provider_types();
        return Err(anyhow::anyhow!("Provider not found"));
    }

    Ok(())
}

/// Configure a provider
pub fn configure_provider(
    provider_name: &str,
    endpoint: Option<&str>,
    api_key: Option<&str>,
    model: Option<&str>,
    enabled: Option<bool>,
) -> Result<()> {
    let provider_type = parse_provider_name(provider_name)?;
    let mut config = CsmConfig::load()?;

    // Get or create provider config
    let mut provider_config = config
        .get_provider(provider_type)
        .cloned()
        .unwrap_or_else(|| ProviderConfig::new(provider_type));

    // Update settings
    if let Some(endpoint) = endpoint {
        provider_config.endpoint = Some(endpoint.to_string());
    }

    if let Some(api_key) = api_key {
        provider_config.api_key = Some(api_key.to_string());
    }

    if let Some(model) = model {
        provider_config.model = Some(model.to_string());
    }

    if let Some(enabled) = enabled {
        provider_config.enabled = enabled;
    }

    // Save config
    config.set_provider(provider_config.clone());
    config.save()?;

    println!("{} Configured provider: {}", "+".green(), provider_name);
    println!();
    println!(
        "  Endpoint: {}",
        provider_config.endpoint.as_deref().unwrap_or("(default)")
    );
    println!(
        "  API Key:  {}",
        if provider_config.api_key.is_some() {
            "(set)".green().to_string()
        } else {
            "(none)".dimmed().to_string()
        }
    );
    println!(
        "  Model:    {}",
        provider_config.model.as_deref().unwrap_or("(default)")
    );
    println!(
        "  Enabled:  {}",
        if provider_config.enabled {
            "Yes".green()
        } else {
            "No".red()
        }
    );

    Ok(())
}

/// Import sessions from another provider
pub fn import_from_provider(
    from_provider: &str,
    target_path: Option<&str>,
    session_id: Option<&str>,
) -> Result<()> {
    let provider_type = parse_provider_name(from_provider)?;
    let registry = ProviderRegistry::new();

    let provider = registry
        .get_provider(provider_type)
        .ok_or_else(|| anyhow::anyhow!("Provider not found: {}", from_provider))?;

    if !provider.is_available() {
        return Err(anyhow::anyhow!(
            "Provider {} is not available",
            from_provider
        ));
    }

    let project_path = target_path
        .map(String::from)
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|p| p.to_string_lossy().to_string())
        })
        .ok_or_else(|| anyhow::anyhow!("Could not determine target path"))?;

    if let Some(session_id) = session_id {
        // Import specific session
        println!(
            "Importing session {} from {}...",
            session_id,
            provider.name()
        );

        let session = provider.import_session(session_id)?;

        // Save to target workspace
        let workspace = crate::workspace::get_workspace_by_path(&project_path)?
            .ok_or_else(|| anyhow::anyhow!("Workspace not found for path: {}", project_path))?;
        let sessions_dir = workspace.chat_sessions_path;
        std::fs::create_dir_all(&sessions_dir)?;

        let session_file = sessions_dir.join(format!("{}.json", session_id));
        let content = serde_json::to_string_pretty(&session)?;
        std::fs::write(&session_file, content)?;

        println!("{} Imported session: {}", "+".green(), session.title());
    } else {
        // Import all sessions
        println!("Importing all sessions from {}...", provider.name());

        let sessions = provider.list_sessions()?;

        if sessions.is_empty() {
            println!("  No sessions found");
            return Ok(());
        }

        let workspace = crate::workspace::get_workspace_by_path(&project_path)?
            .ok_or_else(|| anyhow::anyhow!("Workspace not found for path: {}", project_path))?;
        let sessions_dir = workspace.chat_sessions_path;
        std::fs::create_dir_all(&sessions_dir)?;

        let mut imported = 0;
        for session in &sessions {
            let id = session
                .session_id
                .clone()
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            let session_file = sessions_dir.join(format!("{}.json", id));

            if !session_file.exists() {
                let content = serde_json::to_string_pretty(&session)?;
                std::fs::write(&session_file, content)?;
                imported += 1;
                println!("  {} {}", "+".green(), session.title());
            }
        }

        println!();
        println!(
            "Imported {} of {} sessions",
            imported.to_string().green(),
            sessions.len()
        );
    }

    Ok(())
}

/// Test connection to a provider
pub fn test_provider(provider_name: &str) -> Result<()> {
    let provider_type = parse_provider_name(provider_name)?;
    let registry = ProviderRegistry::new();

    print!("Testing {} connection... ", provider_type.display_name());

    if let Some(provider) = registry.get_provider(provider_type) {
        if provider.is_available() {
            println!("{}", "OK".green());

            // Try to list sessions
            match provider.list_sessions() {
                Ok(sessions) => {
                    println!("  Found {} sessions", sessions.len());
                }
                Err(e) => {
                    println!("  {}: {}", "Warning".yellow(), e);
                }
            }

            Ok(())
        } else {
            println!("{}", "FAILED".red());
            println!();

            if let Some(endpoint) = provider_type.default_endpoint() {
                println!("  Expected endpoint: {}", endpoint);
                println!("  Make sure the service is running.");
            }

            Err(anyhow::anyhow!("Provider not available"))
        }
    } else {
        println!("{}", "NOT FOUND".red());
        Err(anyhow::anyhow!("Provider not found"))
    }
}

/// Parse a provider name string into ProviderType
fn parse_provider_name(name: &str) -> Result<ProviderType> {
    match name.to_lowercase().as_str() {
        "copilot" | "github-copilot" | "vscode" => Ok(ProviderType::Copilot),
        "cursor" => Ok(ProviderType::Cursor),
        "ollama" => Ok(ProviderType::Ollama),
        "vllm" => Ok(ProviderType::Vllm),
        "foundry" | "azure-foundry" | "foundry-local" | "ai-foundry" => Ok(ProviderType::Foundry),
        "openai" => Ok(ProviderType::OpenAI),
        "lm-studio" | "lmstudio" => Ok(ProviderType::LmStudio),
        "localai" | "local-ai" => Ok(ProviderType::LocalAI),
        "text-gen-webui" | "textgenwebui" | "oobabooga" => Ok(ProviderType::TextGenWebUI),
        "jan" | "jan-ai" | "janai" => Ok(ProviderType::Jan),
        "gpt4all" => Ok(ProviderType::Gpt4All),
        "llamafile" => Ok(ProviderType::Llamafile),
        "custom" => Ok(ProviderType::Custom),
        _ => {
            eprintln!("{} Unknown provider: {}", "Error:".red(), name);
            eprintln!();
            list_provider_types();
            Err(anyhow::anyhow!("Unknown provider"))
        }
    }
}

/// Print list of available provider type names
fn list_provider_types() {
    eprintln!("Supported providers:");
    eprintln!("  copilot      - GitHub Copilot (VS Code)");
    eprintln!("  cursor       - Cursor IDE");
    eprintln!("  ollama       - Ollama local models");
    eprintln!("  vllm         - vLLM server");
    eprintln!("  foundry      - Azure AI Foundry / Foundry Local");
    eprintln!("  openai       - OpenAI API");
    eprintln!("  lm-studio    - LM Studio");
    eprintln!("  localai      - LocalAI");
    eprintln!("  text-gen-webui - Text Generation WebUI");
    eprintln!("  jan          - Jan.ai");
    eprintln!("  gpt4all      - GPT4All");
    eprintln!("  llamafile    - Llamafile");
    eprintln!("  custom       - Custom OpenAI-compatible endpoint");
}
