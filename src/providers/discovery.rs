// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Provider discovery utilities

use super::{ProviderRegistry, ProviderType};
use colored::*;

/// Discover all available LLM providers and return a summary
pub fn discover_all_providers() -> ProviderRegistry {
    ProviderRegistry::new()
}

/// Print a summary of discovered providers
pub fn print_provider_summary(registry: &ProviderRegistry) {
    println!("{}", "Discovered LLM Providers:".bold());
    println!();

    let available: Vec<_> = registry.available_providers();
    let all_providers = registry.providers();

    if all_providers.is_empty() {
        println!("  {}", "No providers discovered".dimmed());
        return;
    }

    for provider in all_providers {
        let status = if provider.is_available() {
            "+".green()
        } else {
            "x".red()
        };

        let name = provider.name();
        let provider_type = provider.provider_type();

        print!("  {} {}", status, name.bold());

        if provider.is_available() {
            if let Some(path) = provider.sessions_path() {
                print!(" ({})", path.display().to_string().dimmed());
            }
        } else {
            print!(" {}", "(not available)".dimmed());
        }

        // Show default endpoint for server-based providers
        if provider_type.is_openai_compatible() {
            if let Some(endpoint) = provider_type.default_endpoint() {
                print!(" [{}]", endpoint.dimmed());
            }
        }

        println!();
    }

    println!();
    println!(
        "  {} {} available, {} total",
        "Summary:".bold(),
        available.len().to_string().green(),
        all_providers.len()
    );
}

/// Check if a specific provider is available
pub fn is_provider_available(provider_type: ProviderType) -> bool {
    let registry = ProviderRegistry::new();
    registry
        .get_provider(provider_type)
        .is_some_and(|p| p.is_available())
}

/// Get provider endpoints for display
pub fn get_provider_endpoints() -> Vec<(ProviderType, Option<&'static str>)> {
    vec![
        (ProviderType::Copilot, None),
        (ProviderType::Cursor, None),
        (
            ProviderType::Ollama,
            ProviderType::Ollama.default_endpoint(),
        ),
        (ProviderType::Vllm, ProviderType::Vllm.default_endpoint()),
        (
            ProviderType::Foundry,
            ProviderType::Foundry.default_endpoint(),
        ),
        (
            ProviderType::OpenAI,
            ProviderType::OpenAI.default_endpoint(),
        ),
        (
            ProviderType::LmStudio,
            ProviderType::LmStudio.default_endpoint(),
        ),
        (
            ProviderType::LocalAI,
            ProviderType::LocalAI.default_endpoint(),
        ),
        (
            ProviderType::TextGenWebUI,
            ProviderType::TextGenWebUI.default_endpoint(),
        ),
        (ProviderType::Jan, ProviderType::Jan.default_endpoint()),
        (
            ProviderType::Gpt4All,
            ProviderType::Gpt4All.default_endpoint(),
        ),
        (
            ProviderType::Llamafile,
            ProviderType::Llamafile.default_endpoint(),
        ),
    ]
}
