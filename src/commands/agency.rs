// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Agency (Agent Development Kit) command implementations

use crate::agency::{AgentRole, OrchestrationType};
use anyhow::Result;
use colored::Colorize;

/// List available agents and roles
pub fn list_agents(verbose: bool) -> Result<()> {
    println!(
        "{}",
        "+===================================================================+".cyan()
    );
    println!(
        "{}",
        "|              CSM Agent Development Kit (Agency)                   |".cyan()
    );
    println!(
        "{}",
        "+===================================================================+".cyan()
    );
    println!();

    println!("{}", "[*] Available Agent Roles:".bold());
    println!();

    let roles = [
        (
            "coordinator",
            "[C]",
            "Manages and delegates tasks to other agents",
        ),
        ("researcher", "[R]", "Gathers information and analyzes data"),
        ("coder", "[D]", "Writes and modifies code"),
        ("reviewer", "[V]", "Reviews code and provides feedback"),
        ("executor", "[E]", "Executes commands and tools"),
        ("writer", "[W]", "Creates documentation and content"),
        ("tester", "[T]", "Writes and runs tests"),
        ("custom", "[X]", "User-defined agent with custom behavior"),
    ];

    for (role, icon, desc) in roles {
        if verbose {
            println!(
                "  {} {} {}",
                icon,
                role.green().bold(),
                format!("- {}", desc).dimmed()
            );
        } else {
            println!("  {} {}", icon, role.green());
        }
    }

    if verbose {
        println!();
        println!("{}", "[*] Default Agents:".bold());
        println!();
        println!(
            "  {} {} - General-purpose assistant with planning",
            "[A]".dimmed(),
            "assistant".yellow()
        );
        println!(
            "  {} {} - Research specialist with web search",
            "[R]".dimmed(),
            "researcher".yellow()
        );
        println!(
            "  {} {} - Code generation and modification",
            "[D]".dimmed(),
            "coder".yellow()
        );
        println!(
            "  {} {} - Code review and quality assurance",
            "[V]".dimmed(),
            "reviewer".yellow()
        );
    }

    Ok(())
}

/// Show agent information
pub fn show_agent_info(name: &str) -> Result<()> {
    println!("{}", format!("Agent: {}", name).bold());
    println!();

    // Default agent configurations
    match name.to_lowercase().as_str() {
        "assistant" => {
            println!("  {} {}", "Role:".dimmed(), "custom".green());
            println!(
                "  {} General-purpose AI assistant with planning and reflection",
                "Description:".dimmed()
            );
            println!("  {} gemini-2.0-flash (default)", "Model:".dimmed());
            println!("  {} 0.7", "Temperature:".dimmed());
            println!(
                "  {} planning, reflection, code_execution",
                "Capabilities:".dimmed()
            );
        }
        "researcher" => {
            println!("  {} {}", "Role:".dimmed(), "researcher".green());
            println!(
                "  {} Research specialist with web search capabilities",
                "Description:".dimmed()
            );
            println!("  {} gemini-2.0-flash (default)", "Model:".dimmed());
            println!("  {} 0.5", "Temperature:".dimmed());
            println!(
                "  {} web_search, file_read, knowledge_base",
                "Tools:".dimmed()
            );
        }
        "coder" => {
            println!("  {} {}", "Role:".dimmed(), "coder".green());
            println!(
                "  {} Code generation and modification specialist",
                "Description:".dimmed()
            );
            println!("  {} gemini-2.0-flash (default)", "Model:".dimmed());
            println!("  {} 0.3", "Temperature:".dimmed());
            println!(
                "  {} file_read, file_write, terminal, code_search",
                "Tools:".dimmed()
            );
        }
        "reviewer" => {
            println!("  {} {}", "Role:".dimmed(), "reviewer".green());
            println!(
                "  {} Code review and quality assurance",
                "Description:".dimmed()
            );
            println!("  {} gemini-2.0-flash (default)", "Model:".dimmed());
            println!("  {} 0.2", "Temperature:".dimmed());
            println!("  {} file_read, code_search, lint", "Tools:".dimmed());
        }
        _ => {
            println!(
                "  {} Agent '{}' not found in defaults",
                "[!]".yellow(),
                name
            );
            println!();
            println!("  Use 'csm Agency create {}' to create a new agent", name);
        }
    }

    Ok(())
}

/// List orchestration modes
pub fn list_modes() -> Result<()> {
    println!("{}", "[*] Orchestration Modes:".bold());
    println!();

    let modes = [
        ("single", "[1]", "Traditional single-agent response"),
        (
            "sequential",
            "[>]",
            "Agents execute one after another, passing results forward",
        ),
        (
            "parallel",
            "[!]",
            "Multiple agents work simultaneously on subtasks",
        ),
        ("loop", "[O]", "Agent repeats until a condition is met"),
        (
            "hierarchical",
            "[H]",
            "Lead agent delegates to specialized sub-agents",
        ),
        (
            "swarm",
            "[S]",
            "Multiple agents collaborate with a coordinator",
        ),
        ("debate", "[D]", "Agents debate to reach the best solution"),
    ];

    for (mode, icon, desc) in modes {
        println!("  {} {} - {}", icon, mode.cyan().bold(), desc.dimmed());
    }

    println!();
    println!("{}", "Usage:".dimmed());
    println!("  csm Agency run --orchestration swarm \"Build a web scraper\"");

    Ok(())
}

/// Run an agent with a prompt
pub fn run_agent(
    agent_name: &str,
    prompt: &str,
    model: Option<&str>,
    orchestration: &str,
    verbose: bool,
) -> Result<()> {
    let model_name = model.unwrap_or("gemini-2.0-flash");

    println!("{}", "[*] Starting agent execution...".bold());
    println!();
    println!("  {} {}", "Agent:".dimmed(), agent_name.green());
    println!("  {} {}", "Model:".dimmed(), model_name.yellow());
    println!("  {} {}", "Mode:".dimmed(), orchestration.cyan());
    println!("  {} {}", "Prompt:".dimmed(), prompt);
    println!();

    // Parse orchestration type
    let orch_type = match orchestration.to_lowercase().as_str() {
        "single" => OrchestrationType::Sequential, // Single is just sequential with one agent
        "sequential" => OrchestrationType::Sequential,
        "parallel" => OrchestrationType::Parallel,
        "loop" => OrchestrationType::Loop,
        "hierarchical" => OrchestrationType::Hierarchical,
        "swarm" => OrchestrationType::Hierarchical, // Swarm uses hierarchical orchestration
        _ => {
            println!(
                "{} Unknown orchestration mode '{}', using single",
                "[!]".yellow(),
                orchestration
            );
            OrchestrationType::Sequential
        }
    };

    if verbose {
        println!("{}", "[*] Execution Details:".dimmed());
        println!("  Orchestration Type: {:?}", orch_type);
    }

    // For now, show that the agent would be created and run
    // Full implementation requires async runtime and API keys
    println!(
        "{}",
        "[...] Agent execution requires API keys and async runtime.".dimmed()
    );
    println!(
        "{}",
        "   Use 'csm api serve' to start the backend API,".dimmed()
    );
    println!(
        "{}",
        "   then use csm-web or vscode-extension for full agent execution.".dimmed()
    );
    println!();

    // Show what would happen
    println!("{}", "[*] Execution Plan:".bold());
    match orchestration.to_lowercase().as_str() {
        "single" => {
            println!("  1. {} agent receives prompt", agent_name);
            println!("  2. Agent processes and responds");
        }
        "sequential" => {
            println!("  1. First agent processes prompt");
            println!("  2. Result passed to next agent");
            println!("  3. Continue until all agents complete");
        }
        "parallel" => {
            println!("  1. Task decomposed into subtasks");
            println!("  2. Multiple agents work simultaneously");
            println!("  3. Results merged");
        }
        "swarm" | "hierarchical" => {
            println!("  1. Coordinator analyzes task");
            println!("  2. Tasks delegated to specialists");
            println!("  3. Results collected and synthesized");
        }
        "loop" => {
            println!("  1. Agent processes prompt");
            println!("  2. Check completion condition");
            println!("  3. Repeat if needed (max iterations)");
        }
        _ => {}
    }

    Ok(())
}

/// Create a new agent configuration
pub fn create_agent(
    name: &str,
    role: &str,
    instruction: Option<&str>,
    model: Option<&str>,
) -> Result<()> {
    let role_enum = match role.to_lowercase().as_str() {
        "coordinator" => AgentRole::Coordinator,
        "researcher" => AgentRole::Researcher,
        "coder" => AgentRole::Coder,
        "reviewer" => AgentRole::Reviewer,
        "executor" => AgentRole::Executor,
        "writer" => AgentRole::Writer,
        "analyst" => AgentRole::Analyst,
        "assistant" => AgentRole::Assistant,
        "household" => AgentRole::Household,
        "business" => AgentRole::Business,
        "tester" => AgentRole::Tester,
        _ => AgentRole::Custom,
    };

    let default_instruction = match role_enum {
        AgentRole::Coordinator => "You are a coordinator agent that manages and delegates tasks.",
        AgentRole::Researcher => "You are a research specialist that gathers and analyzes information.",
        AgentRole::Coder => "You are a coding specialist that writes and modifies code.",
        AgentRole::Reviewer => "You are a code reviewer that provides quality feedback.",
        AgentRole::Executor => "You are an executor that runs commands and tools.",
        AgentRole::Writer => "You are a writer that creates documentation and content.",
        AgentRole::Analyst => "You are an analyst that examines data and provides insights.",
        AgentRole::Assistant => "You are a helpful AI assistant.",
        AgentRole::Household => "You are a proactive Household Agent that monitors and solves home problems with user permission. Track bills, maintenance, supplies, smart home devices, and daily household tasks.",
        AgentRole::Business => "You are a proactive Business Agent that monitors and solves work problems with user permission. Optimize calendars, triage emails, prepare for meetings, track deadlines, and coordinate projects.",
        AgentRole::Tester => "You are a testing specialist that creates and runs tests.",
        AgentRole::Custom => "You are a helpful AI assistant.",
    };

    let instruction = instruction.unwrap_or(default_instruction);
    let model = model.unwrap_or("gemini-2.0-flash");

    println!("{}", "[+] Agent Configuration Created:".bold().green());
    println!();
    println!("  {} {}", "Name:".dimmed(), name.cyan().bold());
    println!("  {} {:?}", "Role:".dimmed(), role_enum);
    println!("  {} {}", "Model:".dimmed(), model.yellow());
    println!("  {} {}", "Instruction:".dimmed(), instruction);
    println!();
    println!("{}", "[*] To use this agent:".dimmed());
    println!("   csm Agency run --agent {} \"Your prompt here\"", name);

    Ok(())
}

/// List available tools
pub fn list_tools() -> Result<()> {
    println!("{}", "[*] Available Tools:".bold());
    println!();

    let tools = [
        ("file_read", "[F]", "Read file contents"),
        ("file_write", "[W]", "Write or modify files"),
        ("terminal", "[T]", "Execute shell commands"),
        ("web_search", "[?]", "Search the web for information"),
        ("code_search", "[?]", "Search codebase for symbols"),
        ("knowledge_base", "[K]", "Query knowledge base"),
        ("calculator", "[#]", "Perform calculations"),
        ("http_request", "[H]", "Make HTTP requests"),
    ];

    for (tool, icon, desc) in tools {
        println!("  {} {} - {}", icon, tool.green().bold(), desc.dimmed());
    }

    Ok(())
}

/// Show swarm templates
pub fn list_templates() -> Result<()> {
    println!("{}", "[S] Swarm Templates:".bold());
    println!();

    let templates = [
        (
            "code_review",
            "Code Review Team",
            vec!["coder", "reviewer", "tester"],
        ),
        (
            "research",
            "Research Team",
            vec!["coordinator", "researcher", "writer"],
        ),
        (
            "full_stack",
            "Full Stack Team",
            vec!["coordinator", "coder", "reviewer", "tester"],
        ),
        (
            "content",
            "Content Team",
            vec!["researcher", "writer", "reviewer"],
        ),
    ];

    for (id, name, agents) in templates {
        println!("  {} {}", "[*]".dimmed(), name.cyan().bold());
        println!("     {} {}", "ID:".dimmed(), id.yellow());
        println!("     {} {}", "Agents:".dimmed(), agents.join(", ").green());
        println!();
    }

    println!("{}", "Usage:".dimmed());
    println!("  Select a template in csm-web or vscode-extension to create a swarm.");

    Ok(())
}
