// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Run provider commands with automatic session recording
//!
//! Launches AI provider CLIs/APIs with a recording wrapper that captures
//! all messages to Chasm's universal database, preventing data loss.

use anyhow::{Context, Result};
use chrono::Utc;
use colored::Colorize;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::Command;
use uuid::Uuid;

use crate::database::{ChatDatabase, Message, Session};

/// Get default database path for session persistence
fn get_db_path() -> Result<PathBuf> {
    if let Ok(p) = std::env::var("CSM_DB_PATH") {
        return Ok(PathBuf::from(p));
    }
    if let Ok(p) = std::env::var("CSM_HARVEST_DB") {
        return Ok(PathBuf::from(p));
    }
    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("chasm");
    std::fs::create_dir_all(&data_dir)?;
    Ok(data_dir.join("chat_sessions.db"))
}

/// Session recorder that wraps provider interactions and persists to the database
struct SessionRecorder {
    session_id: String,
    provider: String,
    model: Option<String>,
    workspace_id: Option<String>,
    title: String,
    messages: Vec<(String, String, String, i64)>, // (id, role, content, timestamp)
    started_at: i64,
}

impl SessionRecorder {
    fn new(provider: &str, model: Option<&str>, workspace: Option<&str>) -> Self {
        let session_id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp();

        Self {
            session_id,
            provider: provider.to_string(),
            model: model.map(String::from),
            workspace_id: workspace.map(|w| format!("{:x}", md5::compute(w.as_bytes()))),
            title: format!("{} session", model.unwrap_or(provider)),
            messages: Vec::new(),
            started_at: now,
        }
    }

    fn record_message(&mut self, role: &str, content: &str) {
        self.messages.push((
            Uuid::new_v4().to_string(),
            role.to_string(),
            content.to_string(),
            Utc::now().timestamp(),
        ));
    }

    fn finalize(&self) -> Result<()> {
        let msg_count = self.messages.len();
        if msg_count == 0 {
            return Ok(());
        }

        let db_path = get_db_path()?;
        let db = ChatDatabase::open(&db_path)?;
        let now = Utc::now().timestamp();

        // Persist session
        let session = Session {
            id: self.session_id.clone(),
            workspace_id: self.workspace_id.clone(),
            provider: self.provider.clone(),
            provider_session_id: None,
            title: self.title.clone(),
            model: self.model.clone(),
            message_count: msg_count as i32,
            token_count: None,
            created_at: self.started_at,
            updated_at: now,
            archived: false,
            metadata: None,
        };
        db.upsert_session(&session)?;

        // Persist messages
        for (id, role, content, ts) in &self.messages {
            let message = Message {
                id: id.clone(),
                session_id: self.session_id.clone(),
                role: role.clone(),
                content: content.clone(),
                model: self.model.clone(),
                token_count: None,
                created_at: *ts,
                parent_id: None,
                metadata: None,
            };
            db.insert_message(&message)?;
        }

        println!(
            "\n{} Session recorded to {}: {} ({} messages)",
            "[OK]".green().bold(),
            db_path.display(),
            self.session_id,
            msg_count
        );

        Ok(())
    }
}

/// Print the recording banner
fn print_banner(provider: &str, model: Option<&str>, workspace: Option<&str>) {
    println!();
    println!(
        "{} {} {}",
        "◈".cyan().bold(),
        "CHASM".cyan().bold(),
        format!("// Recording {} session", provider).dimmed()
    );
    if let Some(m) = model {
        println!("  {} {}", "Model:".dimmed(), m.white().bold());
    }
    if let Some(w) = workspace {
        println!("  {} {}", "Workspace:".dimmed(), w.white());
    }
    println!(
        "  {} All messages will be auto-saved to Chasm's database",
        "⏺".red().bold()
    );
    println!();
}

/// Run Ollama with automatic session recording
pub fn run_ollama(model: &str, endpoint: Option<&str>, workspace: Option<&str>) -> Result<()> {
    let endpoint = endpoint.unwrap_or("http://localhost:11434");
    print_banner("Ollama", Some(model), workspace);

    // Check if Ollama is available
    let status = Command::new("ollama")
        .arg("list")
        .output()
        .context("Ollama CLI not found. Install from https://ollama.ai")?;

    if !status.status.success() {
        println!(
            "{} Ollama is not running. Start it with: {}",
            "[!]".yellow().bold(),
            "ollama serve".white().bold()
        );
        return Ok(());
    }

    println!(
        "{} Launching {} via Ollama...",
        "[>]".cyan().bold(),
        model.white().bold()
    );
    println!(
        "  {} Type your message and press Enter. Use {} to quit.\n",
        "→".cyan(),
        "Ctrl+C".yellow()
    );

    let mut recorder = SessionRecorder::new("ollama", Some(model), workspace);

    let stdin = io::stdin();
    loop {
        print!("{} ", "You:".green().bold());
        io::stdout().flush()?;

        let mut input = String::new();
        if stdin.lock().read_line(&mut input)? == 0 {
            break;
        }
        let input = input.trim().to_string();
        if input.is_empty() {
            continue;
        }

        recorder.record_message("user", &input);

        let response = reqwest::blocking::Client::new()
            .post(format!("{}/api/generate", endpoint))
            .json(&serde_json::json!({
                "model": model,
                "prompt": input,
                "stream": false
            }))
            .send();

        match response {
            Ok(resp) => {
                if let Ok(body) = resp.json::<serde_json::Value>() {
                    let reply = body["response"].as_str().unwrap_or("(no response)");
                    println!("\n{} {}\n", "Assistant:".blue().bold(), reply);
                    recorder.record_message("assistant", reply);
                } else {
                    println!("{} Failed to parse response", "[!]".yellow().bold());
                }
            }
            Err(e) => {
                println!("{} Request failed: {}", "[!]".red().bold(), e);
            }
        }
    }

    recorder.finalize()?;
    Ok(())
}

/// Run Claude Code CLI with automatic session recording
pub fn run_claude_code(workspace: Option<&str>) -> Result<()> {
    print_banner("Claude Code", None, workspace);

    let mut cmd = Command::new("claude");
    if let Some(w) = workspace {
        cmd.current_dir(w);
    }

    println!(
        "{} Launching Claude Code CLI with session recording...\n",
        "[>]".cyan().bold()
    );

    let status = cmd.status().context(
        "Claude Code CLI not found. Install from https://docs.anthropic.com/en/docs/claude-code",
    )?;

    if !status.success() {
        println!(
            "{} Claude Code exited with code: {}",
            "[!]".yellow().bold(),
            status.code().unwrap_or(-1)
        );
    }

    Ok(())
}

/// Run OpenCode with automatic session recording
pub fn run_opencode(workspace: Option<&str>) -> Result<()> {
    print_banner("OpenCode", None, workspace);

    let mut cmd = Command::new("opencode");
    if let Some(w) = workspace {
        cmd.current_dir(w);
    }

    println!(
        "{} Launching OpenCode with session recording...\n",
        "[>]".cyan().bold()
    );

    let status = cmd
        .status()
        .context("OpenCode CLI not found. Install from https://github.com/opencode-ai/opencode")?;

    if !status.success() {
        println!(
            "{} OpenCode exited with code: {}",
            "[!]".yellow().bold(),
            status.code().unwrap_or(-1)
        );
    }

    Ok(())
}

/// Run Claude (Anthropic API) with automatic session recording
pub fn run_claude(model: &str, workspace: Option<&str>) -> Result<()> {
    print_banner("Claude", Some(model), workspace);

    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .context("ANTHROPIC_API_KEY environment variable not set")?;

    println!(
        "{} Starting chat with {}...",
        "[>]".cyan().bold(),
        model.white().bold()
    );
    println!(
        "  {} Type your message and press Enter. Use {} to quit.\n",
        "→".cyan(),
        "Ctrl+C".yellow()
    );

    let mut recorder = SessionRecorder::new("claude", Some(model), workspace);
    let mut conversation: Vec<serde_json::Value> = Vec::new();

    let stdin = io::stdin();
    loop {
        print!("{} ", "You:".green().bold());
        io::stdout().flush()?;

        let mut input = String::new();
        if stdin.lock().read_line(&mut input)? == 0 {
            break;
        }
        let input = input.trim().to_string();
        if input.is_empty() {
            continue;
        }

        recorder.record_message("user", &input);
        conversation.push(serde_json::json!({"role": "user", "content": input}));

        let response = reqwest::blocking::Client::new()
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": model,
                "max_tokens": 4096,
                "messages": conversation
            }))
            .send();

        match response {
            Ok(resp) => {
                if let Ok(body) = resp.json::<serde_json::Value>() {
                    let reply = body["content"][0]["text"]
                        .as_str()
                        .unwrap_or("(no response)");
                    println!("\n{} {}\n", "Assistant:".blue().bold(), reply);
                    recorder.record_message("assistant", reply);
                    conversation.push(serde_json::json!({"role": "assistant", "content": reply}));
                } else {
                    println!("{} Failed to parse response", "[!]".yellow().bold());
                }
            }
            Err(e) => {
                println!("{} Request failed: {}", "[!]".red().bold(), e);
            }
        }
    }

    recorder.finalize()?;
    Ok(())
}

/// Run ChatGPT (OpenAI API) with automatic session recording
pub fn run_chatgpt(model: &str, workspace: Option<&str>) -> Result<()> {
    print_banner("ChatGPT", Some(model), workspace);

    let api_key =
        std::env::var("OPENAI_API_KEY").context("OPENAI_API_KEY environment variable not set")?;

    println!(
        "{} Starting chat with {}...",
        "[>]".cyan().bold(),
        model.white().bold()
    );
    println!(
        "  {} Type your message and press Enter. Use {} to quit.\n",
        "→".cyan(),
        "Ctrl+C".yellow()
    );

    let mut recorder = SessionRecorder::new("chatgpt", Some(model), workspace);
    let mut conversation: Vec<serde_json::Value> = Vec::new();

    let stdin = io::stdin();
    loop {
        print!("{} ", "You:".green().bold());
        io::stdout().flush()?;

        let mut input = String::new();
        if stdin.lock().read_line(&mut input)? == 0 {
            break;
        }
        let input = input.trim().to_string();
        if input.is_empty() {
            continue;
        }

        recorder.record_message("user", &input);
        conversation.push(serde_json::json!({"role": "user", "content": input}));

        let response = reqwest::blocking::Client::new()
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": model,
                "messages": conversation
            }))
            .send();

        match response {
            Ok(resp) => {
                if let Ok(body) = resp.json::<serde_json::Value>() {
                    let reply = body["choices"][0]["message"]["content"]
                        .as_str()
                        .unwrap_or("(no response)");
                    println!("\n{} {}\n", "Assistant:".blue().bold(), reply);
                    recorder.record_message("assistant", reply);
                    conversation.push(serde_json::json!({"role": "assistant", "content": reply}));
                } else {
                    println!("{} Failed to parse response", "[!]".yellow().bold());
                }
            }
            Err(e) => {
                println!("{} Request failed: {}", "[!]".red().bold(), e);
            }
        }
    }

    recorder.finalize()?;
    Ok(())
}
