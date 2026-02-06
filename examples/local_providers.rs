//! Local provider examples for CSM library
//!
//! Demonstrates working with local LLM providers:
//! - VS Code Copilot Chat
//! - Cursor
//! - Ollama
//! - LM Studio
//! - vLLM
//! - LocalAI
//! - GPT4All
//!
//! Run with: cargo run --example local_providers

use chasm::providers::discovery::discover_all_providers;
use chasm::providers::ollama::OllamaProvider;
use chasm::providers::openai_compat::OpenAICompatProvider;
use chasm::providers::{
    ChatProvider, CsmConfig, GenericMessage, GenericSession, ProviderConfig, ProviderType,
};
use chasm::workspace::discover_workspaces;

fn main() -> anyhow::Result<()> {
    println!("=== CSM Local Provider Examples ===\n");

    // ========================================================================
    // Example 1: Discover all available local providers
    // ========================================================================
    println!("1. Discovering available local providers...");

    let registry = discover_all_providers();
    let providers = registry.providers();

    println!("   Found {} providers:", providers.len());
    for provider in providers {
        let status = if provider.is_available() { "+" } else { "x" };
        let endpoint = provider
            .sessions_path()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "(no path)".to_string());
        println!("   {} {} - {}", status, provider.name(), endpoint);
    }

    // ========================================================================
    // Example 2: Check which providers use file storage vs API
    // ========================================================================
    println!("\n2. Provider storage types:");

    let file_based = vec![ProviderType::Copilot, ProviderType::Cursor];

    let api_based = vec![
        ProviderType::Ollama,
        ProviderType::Vllm,
        ProviderType::LmStudio,
        ProviderType::LocalAI,
        ProviderType::Foundry,
        ProviderType::Gpt4All,
        ProviderType::Jan,
        ProviderType::Llamafile,
    ];

    println!("   File-based (stores sessions locally):");
    for pt in &file_based {
        println!(
            "     - {} (uses: {})",
            pt.display_name(),
            if pt.uses_file_storage() {
                "file storage"
            } else {
                "API"
            }
        );
    }

    println!("   API-based (OpenAI compatible):");
    for pt in &api_based {
        println!(
            "     - {} @ {}",
            pt.display_name(),
            pt.default_endpoint().unwrap_or("(no default)")
        );
    }

    // ========================================================================
    // Example 3: Working with Copilot Chat sessions
    // ========================================================================
    println!("\n3. VS Code Copilot Chat sessions:");

    let workspaces = discover_workspaces()?;
    let mut total_sessions = 0;

    for ws in workspaces.iter().take(5) {
        if ws.chat_session_count > 0 {
            println!(
                "   {} - {} sessions",
                ws.project_path.as_deref().unwrap_or(&ws.hash[..12]),
                ws.chat_session_count
            );
            total_sessions += ws.chat_session_count;
        }
    }
    println!("   Total sessions (first 5 workspaces): {}", total_sessions);

    // ========================================================================
    // Example 4: Create a provider configuration
    // ========================================================================
    println!("\n4. Creating provider configurations:");

    // Ollama configuration
    let mut ollama_config = ProviderConfig::new(ProviderType::Ollama);
    ollama_config.endpoint = Some("http://localhost:11434".to_string());
    ollama_config.model = Some("llama2".to_string());

    println!("   Ollama config:");
    println!("     Type: {:?}", ollama_config.provider_type);
    println!("     Endpoint: {:?}", ollama_config.endpoint);
    println!("     Model: {:?}", ollama_config.model);

    // LM Studio configuration
    let mut lmstudio_config = ProviderConfig::new(ProviderType::LmStudio);
    lmstudio_config.endpoint = Some("http://localhost:1234/v1".to_string());
    lmstudio_config.model = Some("local-model".to_string());

    println!("   LM Studio config:");
    println!("     Type: {:?}", lmstudio_config.provider_type);
    println!("     Endpoint: {:?}", lmstudio_config.endpoint);

    // ========================================================================
    // Example 5: Discover and configure an Ollama provider
    // ========================================================================
    println!("\n5. Ollama provider setup:");

    // Discover Ollama (checks if it's installed/running)
    if let Some(ollama) = OllamaProvider::discover() {
        println!("   Provider: {}", ollama.name());
        println!("   Type: {:?}", ollama.provider_type());
        println!("   Available: {}", ollama.is_available());

        if ollama.is_available() {
            println!("   + Ollama server is running");
        } else {
            println!("   x Ollama server not detected (start with: ollama serve)");
        }
    } else {
        println!("   Ollama not found on this system");
    }

    // ========================================================================
    // Example 6: OpenAI-compatible provider (works with many local servers)
    // ========================================================================
    println!("\n6. OpenAI-compatible provider setup:");

    // This works with: LM Studio, vLLM, LocalAI, Ollama (with OpenAI mode), etc.
    let openai_compat = OpenAICompatProvider::new(
        ProviderType::LmStudio,
        "LM Studio",
        "http://localhost:1234/v1",
    );

    println!("   Provider: {}", openai_compat.name());
    println!("   Type: {:?}", openai_compat.provider_type());
    println!("   Compatible with:");
    println!("     - LM Studio (http://localhost:1234/v1)");
    println!("     - vLLM (http://localhost:8000/v1)");
    println!("     - LocalAI (http://localhost:8080/v1)");
    println!("     - Ollama OpenAI mode (http://localhost:11434/v1)");

    // ========================================================================
    // Example 7: Convert sessions to generic format for transfer
    // ========================================================================
    println!("\n7. Session format conversion:");

    // Create a sample generic session (portable format)
    let generic_session = GenericSession {
        id: "local-session-001".to_string(),
        title: Some("Rust Programming Help".to_string()),
        messages: vec![
            GenericMessage {
                role: "user".to_string(),
                content: "How do I create a vector in Rust?".to_string(),
                timestamp: Some(chrono::Utc::now().timestamp_millis()),
                model: None,
            },
            GenericMessage {
                role: "assistant".to_string(),
                content: "You can create a vector using `Vec::new()` or the `vec![]` macro:\n\n```rust\nlet v1: Vec<i32> = Vec::new();\nlet v2 = vec![1, 2, 3];\n```".to_string(),
                timestamp: Some(chrono::Utc::now().timestamp_millis()),
                model: Some("llama2".to_string()),
            },
        ],
        created_at: Some(chrono::Utc::now().timestamp_millis()),
        updated_at: Some(chrono::Utc::now().timestamp_millis()),
        provider: Some("Ollama".to_string()),
        model: Some("llama2".to_string()),
    };

    println!("   Generic session: {}", generic_session.id);
    println!("   Title: {:?}", generic_session.title);
    println!("   Messages: {}", generic_session.messages.len());
    println!("   Provider: {:?}", generic_session.provider);

    // Convert to VS Code ChatSession format
    let chat_session: chasm::models::ChatSession = generic_session.clone().into();
    println!("\n   Converted to ChatSession:");
    println!("   Session ID: {:?}", chat_session.session_id);
    println!("   Requests: {}", chat_session.requests.len());
    println!("   Is imported: {}", chat_session.is_imported);

    // ========================================================================
    // Example 8: CSM configuration with multiple providers
    // ========================================================================
    println!("\n8. Multi-provider CSM configuration:");

    let mut ollama_cfg = ProviderConfig::new(ProviderType::Ollama);
    ollama_cfg.endpoint = Some("http://localhost:11434".to_string());
    ollama_cfg.model = Some("codellama".to_string());

    let mut lmstudio_cfg = ProviderConfig::new(ProviderType::LmStudio);
    lmstudio_cfg.endpoint = Some("http://localhost:1234/v1".to_string());

    let config = CsmConfig {
        default_provider: Some(ProviderType::Copilot),
        providers: vec![
            ProviderConfig::new(ProviderType::Copilot),
            ollama_cfg,
            lmstudio_cfg,
        ],
        auto_discover: true,
    };

    println!("   Default provider: {:?}", config.default_provider);
    println!("   Configured providers:");
    for p in &config.providers {
        println!(
            "     - {} @ {}",
            p.provider_type.display_name(),
            p.endpoint.as_deref().unwrap_or("(default)")
        );
    }

    // Serialize to JSON
    let config_json = serde_json::to_string_pretty(&config)?;
    println!("\n   Configuration JSON:");
    for line in config_json.lines().take(10) {
        println!("   {}", line);
    }
    println!("   ...");

    println!("\n=== Local Provider Examples Complete ===");
    Ok(())
}
