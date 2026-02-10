// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! CLI argument definitions using clap derive macros

use clap::{Parser, Subcommand};

/// Chat System Manager (csm) - Manage and merge chat sessions across workspaces
#[derive(Parser)]
#[command(name = "csm")]
#[command(author = "Nervosys")]
#[command(version)]
#[command(about = "Manage and merge chat sessions across workspaces", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    // ============================================================================
    // List Commands
    // ============================================================================
    /// List workspaces, sessions, or paths
    #[command(visible_alias = "ls")]
    List {
        #[command(subcommand)]
        command: Option<ListCommands>,
    },

    // ============================================================================
    // Find Commands
    // ============================================================================
    /// Search workspaces or sessions by text pattern (title, content, ID)
    Find {
        #[command(subcommand)]
        command: Option<FindCommands>,
    },

    // ============================================================================
    // Show Commands
    // ============================================================================
    /// Show workspaces, sessions, or paths
    #[command(visible_alias = "info")]
    Show {
        #[command(subcommand)]
        command: Option<ShowCommands>,
    },

    // ============================================================================
    // Fetch Commands
    // ============================================================================
    /// Fetch chat sessions from workspaces, sessions, or paths
    Fetch {
        #[command(subcommand)]
        command: Option<FetchCommands>,
    },

    // ============================================================================
    // Merge Commands
    // ============================================================================
    /// Merge chat sessions from workspaces, sessions, or paths
    Merge {
        #[command(subcommand)]
        command: Option<MergeCommands>,
    },

    // ============================================================================
    // Export Commands
    // ============================================================================
    /// Export chat sessions from workspaces, sessions, or paths
    Export {
        #[command(subcommand)]
        command: Option<ExportCommands>,
    },

    // ============================================================================
    // Import Commands
    // ============================================================================
    /// Import session files from external directories into a workspace
    Import {
        #[command(subcommand)]
        command: Option<ImportCommands>,
    },

    // ============================================================================
    // Move Commands
    // ============================================================================
    /// Move chat sessions between workspaces
    #[command(visible_alias = "mv")]
    Move {
        #[command(subcommand)]
        command: Option<MoveCommands>,
    },

    // ============================================================================
    // Git Integration Commands
    // ============================================================================
    /// Git integration for chat session versioning
    Git {
        #[command(subcommand)]
        command: GitCommands,
    },

    // ============================================================================
    // Migration Commands
    // ============================================================================
    /// Migration commands for moving chat sessions between machines
    Migration {
        #[command(subcommand)]
        command: MigrationCommands,
    },

    // ============================================================================
    // Run Commands (TUI)
    // ============================================================================
    /// Run interactive tools
    Run {
        #[command(subcommand)]
        command: RunCommands,
    },

    // ============================================================================
    // Provider Commands
    // ============================================================================
    /// Manage LLM providers (Ollama, vLLM, Foundry, Cursor, etc.)
    Provider {
        #[command(subcommand)]
        command: ProviderCommands,
    },

    // ============================================================================
    // Detect Commands
    // ============================================================================
    /// Auto-detect workspace and provider information
    Detect {
        #[command(subcommand)]
        command: Option<DetectCommands>,
    },

    // ============================================================================
    // Register Commands
    // ============================================================================
    /// Add on-disk sessions to VS Code's database index (makes orphaned sessions visible)
    #[command(visible_alias = "sync")]
    Register {
        #[command(subcommand)]
        command: RegisterCommands,
    },

    // ============================================================================
    // Harvest Commands
    // ============================================================================
    /// Harvest chat sessions from all providers into a unified database
    Harvest {
        #[command(subcommand)]
        command: HarvestCommands,
    },

    // ============================================================================
    // Recover Commands
    // ============================================================================
    /// Recover lost chat sessions from backups, recording state, or corrupted files
    #[command(visible_alias = "restore")]
    Recover {
        #[command(subcommand)]
        command: RecoverCommands,
    },

    // ============================================================================
    // API Server Commands
    // ============================================================================
    /// Start the HTTP API server for the web frontend
    #[command(visible_alias = "serve")]
    Api {
        #[command(subcommand)]
        command: ApiCommands,
    },

    // ============================================================================
    // Agency Commands
    // ============================================================================
    /// Agent Development Kit - manage agents and orchestration
    Agency {
        #[command(subcommand)]
        command: AgencyCommands,
    },

    // ============================================================================
    // Telemetry Commands
    // ============================================================================
    /// Manage anonymous usage data collection (opt-in by default)
    Telemetry {
        #[command(subcommand)]
        command: Option<TelemetryCommands>,
    },

    // ============================================================================
    // Easter Egg
    // ============================================================================
    /// Show banner
    #[command(hide = true)]
    Banner,
}

// ============================================================================
// List Subcommands
// ============================================================================

#[derive(Subcommand)]
pub enum ListCommands {
    /// List all VS Code workspaces
    #[command(visible_alias = "ws")]
    Workspaces,

    /// List all chat sessions
    #[command(visible_alias = "s")]
    Sessions {
        /// Filter by project path
        #[arg(long)]
        project_path: Option<String>,

        /// Show file sizes
        #[arg(long, short = 's')]
        size: bool,

        /// Filter by provider (vscode, cursor, claudecode, opencode, openclaw, antigravity)
        #[arg(long, short = 'p')]
        provider: Option<String>,

        /// Include all providers
        #[arg(long)]
        all_providers: bool,
    },

    /// List agent mode sessions (Copilot Edits / chatEditingSessions)
    #[command(visible_alias = "a")]
    Agents {
        /// Filter by project path
        #[arg(long)]
        project_path: Option<String>,

        /// Show file sizes
        #[arg(long, short = 's')]
        size: bool,

        /// Filter by provider (vscode, cursor, claudecode, opencode, openclaw, antigravity)
        #[arg(long, short = 'p')]
        provider: Option<String>,
    },

    /// List sessions for a specific project path
    Path {
        /// Project path (default: current directory)
        project_path: Option<String>,
    },

    /// List unregistered sessions (exist on disk but invisible to VS Code)
    Orphaned {
        /// Project path (default: current directory)
        #[arg(long)]
        path: Option<String>,
    },
}

// ============================================================================
// Find Subcommands
// ============================================================================

#[derive(Subcommand)]
pub enum FindCommands {
    /// Search workspaces by name pattern (defaults to current directory name)
    #[command(visible_alias = "ws")]
    Workspace {
        /// Text pattern to match (case-insensitive, defaults to current directory name)
        pattern: Option<String>,
    },

    /// Search sessions by title, content, or ID pattern
    #[command(visible_alias = "s")]
    Session {
        /// Text pattern to match (case-insensitive, defaults to current directory name)
        pattern: Option<String>,

        /// Filter by project path or workspace name
        #[arg(long, short = 'w')]
        workspace: Option<String>,

        /// Only search in session titles (faster, skip content search)
        #[arg(long, short = 't')]
        title_only: bool,

        /// Include message content in search (slower)
        #[arg(long, short = 'c')]
        content: bool,

        /// Filter sessions modified after this date (YYYY-MM-DD)
        #[arg(long)]
        after: Option<String>,

        /// Filter sessions modified before this date (YYYY-MM-DD)
        #[arg(long)]
        before: Option<String>,

        /// Filter by internal message timestamp date (YYYY-MM-DD)
        #[arg(long)]
        date: Option<String>,

        /// Search across all workspaces (not just current project)
        #[arg(long, short = 'a')]
        all: bool,

        /// Filter by provider (vscode, cursor, claudecode, opencode, openclaw, antigravity)
        #[arg(long, short = 'p')]
        provider: Option<String>,

        /// Search across all providers
        #[arg(long)]
        all_providers: bool,

        /// Limit number of results
        #[arg(long, short = 'n', default_value = "50")]
        limit: usize,
    },

    /// Search sessions within a specific project path
    Path {
        /// Search pattern (case-insensitive, defaults to current directory name)
        pattern: Option<String>,

        /// Project path (default: current directory)
        #[arg(long)]
        project_path: Option<String>,
    },
}

// ============================================================================
// Show Subcommands
// ============================================================================

#[derive(Subcommand)]
pub enum ShowCommands {
    /// Show workspace details
    #[command(visible_alias = "ws")]
    Workspace {
        /// Workspace name or hash
        workspace: String,
    },

    /// Show session details
    #[command(visible_alias = "s")]
    Session {
        /// Session ID or filename
        session_id: String,

        /// Project path to search in
        #[arg(long)]
        project_path: Option<String>,
    },

    /// Show agent mode session details (Copilot Edits)
    #[command(visible_alias = "a")]
    Agent {
        /// Agent session ID
        session_id: String,

        /// Project path to search in
        #[arg(long)]
        project_path: Option<String>,
    },

    /// Show chat history timeline for a project path
    Path {
        /// Path to the project (default: current directory)
        project_path: Option<String>,
    },

    /// Show timeline of session activity with gaps visualization
    Timeline {
        /// Path to the project (default: current directory)
        project_path: Option<String>,

        /// Include agent mode sessions
        #[arg(long, short = 'a')]
        agents: bool,

        /// Filter by provider (vscode, cursor, claudecode, opencode, openclaw, antigravity)
        #[arg(long, short = 'p')]
        provider: Option<String>,

        /// Include all providers (aggregate timeline)
        #[arg(long)]
        all_providers: bool,
    },
}

// ============================================================================
// Fetch Subcommands
// ============================================================================

#[derive(Subcommand)]
pub enum FetchCommands {
    /// Fetch sessions from workspaces matching a pattern
    #[command(visible_alias = "ws")]
    Workspace {
        /// Workspace name pattern (case-insensitive)
        workspace_name: String,

        /// Target project path (default: current directory)
        #[arg(long)]
        target_path: Option<String>,

        /// Overwrite existing sessions
        #[arg(long)]
        force: bool,

        /// Don't register sessions in VS Code index
        #[arg(long)]
        no_register: bool,
    },

    /// Fetch specific sessions by ID
    #[command(visible_alias = "s")]
    Session {
        /// Session IDs to fetch (space-separated)
        #[arg(required = true, num_args = 1..)]
        session_ids: Vec<String>,

        /// Target project path (default: current directory)
        #[arg(long)]
        target_path: Option<String>,

        /// Overwrite existing sessions
        #[arg(long)]
        force: bool,

        /// Don't register sessions in VS Code index
        #[arg(long)]
        no_register: bool,
    },

    /// Fetch chat sessions from other workspaces by project path
    Path {
        /// Path to the project (default: current directory)
        project_path: Option<String>,

        /// Overwrite existing sessions and skip VS Code running check
        #[arg(long)]
        force: bool,

        /// Don't register sessions in VS Code index
        #[arg(long)]
        no_register: bool,
    },
}

// ============================================================================
// Merge Subcommands
// ============================================================================

#[derive(Subcommand)]
pub enum MergeCommands {
    /// Merge sessions from workspaces matching a name pattern
    #[command(visible_alias = "ws")]
    Workspace {
        /// Workspace name pattern to search for (case-insensitive)
        workspace_name: String,

        /// Title for the merged session
        #[arg(short, long)]
        title: Option<String>,

        /// Target project path to save the merged session (default: current directory)
        #[arg(long)]
        target_path: Option<String>,

        /// Skip VS Code running check
        #[arg(long)]
        force: bool,

        /// Don't create backup of current sessions
        #[arg(long)]
        no_backup: bool,
    },

    /// Merge sessions from multiple workspace name patterns
    #[command(visible_alias = "wss")]
    Workspaces {
        /// Workspace name patterns to search for (space-separated, case-insensitive)
        #[arg(required = true, num_args = 1..)]
        workspace_names: Vec<String>,

        /// Title for the merged session
        #[arg(short, long)]
        title: Option<String>,

        /// Target project path to save the merged session (default: current directory)
        #[arg(long)]
        target_path: Option<String>,

        /// Skip VS Code running check
        #[arg(long)]
        force: bool,

        /// Don't create backup of current sessions
        #[arg(long)]
        no_backup: bool,
    },

    /// Merge specific sessions by their IDs or filenames
    #[command(visible_alias = "s")]
    Sessions {
        /// Session IDs or filenames (comma-separated or space-separated)
        #[arg(required = true, num_args = 1..)]
        sessions: Vec<String>,

        /// Title for the merged session
        #[arg(short, long)]
        title: Option<String>,

        /// Target project path to save the merged session (default: current directory)
        #[arg(long)]
        target_path: Option<String>,

        /// Skip VS Code running check
        #[arg(long)]
        force: bool,

        /// Don't create backup of current sessions
        #[arg(long)]
        no_backup: bool,
    },

    /// Merge all sessions for a project path into one unified chat
    Path {
        /// Path to the project (default: current directory)
        project_path: Option<String>,

        /// Title for the merged session
        #[arg(short, long)]
        title: Option<String>,

        /// Skip VS Code running check
        #[arg(long)]
        force: bool,

        /// Don't create backup of current sessions
        #[arg(long)]
        no_backup: bool,
    },

    /// Merge sessions from an LLM provider (Ollama, Cursor, etc.)
    Provider {
        /// Provider name (copilot, cursor, ollama, vllm, foundry, etc.)
        provider_name: String,

        /// Title for the merged session
        #[arg(short, long)]
        title: Option<String>,

        /// Target project path to save the merged session (default: current directory)
        #[arg(long)]
        target_path: Option<String>,

        /// Session IDs from the provider to include (omit for all)
        #[arg(long)]
        sessions: Option<Vec<String>>,

        /// Skip VS Code running check
        #[arg(long)]
        force: bool,

        /// Don't create backup of current sessions
        #[arg(long)]
        no_backup: bool,
    },

    /// Merge sessions from multiple providers
    #[command(name = "providers")]
    Providers {
        /// Provider names (space-separated: copilot cursor ollama)
        #[arg(required = true, num_args = 1..)]
        providers: Vec<String>,

        /// Title for the merged session
        #[arg(short, long)]
        title: Option<String>,

        /// Target project path to save the merged session (default: current directory)
        #[arg(long)]
        target_path: Option<String>,

        /// Filter by workspace name pattern (applies to providers that support workspaces)
        #[arg(long)]
        workspace: Option<String>,

        /// Skip VS Code running check
        #[arg(long)]
        force: bool,

        /// Don't create backup of current sessions
        #[arg(long)]
        no_backup: bool,
    },

    /// Merge all sessions across all available providers
    All {
        /// Title for the merged session
        #[arg(short, long)]
        title: Option<String>,

        /// Target project path to save the merged session (default: current directory)
        #[arg(long)]
        target_path: Option<String>,

        /// Filter by workspace name pattern (applies to providers that support workspaces)
        #[arg(long)]
        workspace: Option<String>,

        /// Skip VS Code running check
        #[arg(long)]
        force: bool,

        /// Don't create backup of current sessions
        #[arg(long)]
        no_backup: bool,
    },
}

// ============================================================================
// Export Subcommands
// ============================================================================

#[derive(Subcommand)]
pub enum ExportCommands {
    /// Export sessions from a workspace by hash
    #[command(visible_alias = "ws")]
    Workspace {
        /// Destination directory for exported sessions
        destination: String,

        /// Source workspace hash
        hash: String,
    },

    /// Export specific sessions by ID
    #[command(visible_alias = "s")]
    Sessions {
        /// Destination directory for exported sessions
        destination: String,

        /// Session IDs to export (space-separated)
        #[arg(required = true, num_args = 1..)]
        session_ids: Vec<String>,

        /// Source project path
        #[arg(long)]
        project_path: Option<String>,
    },

    /// Export chat sessions from a project path
    Path {
        /// Destination directory for exported sessions
        destination: String,

        /// Source project path (default: current directory)
        project_path: Option<String>,
    },
}

// ============================================================================
// Import Subcommands
// ============================================================================

#[derive(Subcommand)]
pub enum ImportCommands {
    /// Copy session files from external directory into a workspace
    #[command(visible_alias = "ws")]
    Workspace {
        /// Source directory containing session JSON files to import
        source: String,

        /// Target workspace hash
        hash: String,

        /// Overwrite existing sessions
        #[arg(long)]
        force: bool,
    },

    /// Copy specific session files into a workspace
    #[command(visible_alias = "s")]
    Sessions {
        /// Session files to import (space-separated paths)
        #[arg(required = true, num_args = 1..)]
        session_files: Vec<String>,

        /// Target project path (default: current directory)
        #[arg(long)]
        target_path: Option<String>,

        /// Overwrite existing sessions
        #[arg(long)]
        force: bool,
    },

    /// Copy session files from external directory into a project workspace
    Path {
        /// Source directory containing session JSON files to import
        source: String,

        /// Target project path (default: current directory)
        target_path: Option<String>,

        /// Overwrite existing sessions
        #[arg(long)]
        force: bool,
    },
}

// ============================================================================
// Move Subcommands
// ============================================================================

#[derive(Subcommand)]
pub enum MoveCommands {
    /// Move all sessions from one workspace to another
    #[command(visible_alias = "ws")]
    Workspace {
        /// Source workspace hash
        source_hash: String,

        /// Target workspace hash or project path
        target: String,
    },

    /// Move specific sessions by ID
    #[command(visible_alias = "s")]
    Sessions {
        /// Session IDs to move (space-separated)
        #[arg(required = true, num_args = 1..)]
        session_ids: Vec<String>,

        /// Target project path
        target_path: String,
    },

    /// Move sessions from a source path to target path
    Path {
        /// Source project path
        source_path: String,

        /// Target project path
        target_path: String,
    },
}

// ============================================================================
// Git Subcommands
// ============================================================================

#[derive(Subcommand)]
pub enum GitCommands {
    /// Configure git settings for chat sessions
    Config {
        /// Git user name
        #[arg(long)]
        name: Option<String>,

        /// Git user email
        #[arg(long)]
        email: Option<String>,

        /// Project path
        #[arg(long)]
        path: Option<String>,
    },

    /// Initialize git versioning for chat sessions
    Init {
        /// Project path
        path: String,
    },

    /// Add chat sessions to git (stage and optionally commit)
    Add {
        /// Project path
        path: String,

        /// Also commit the changes
        #[arg(long)]
        commit: bool,

        /// Commit message (requires --commit)
        #[arg(short, long)]
        message: Option<String>,
    },

    /// Show git status of chat sessions
    Status {
        /// Project path
        path: String,
    },

    /// Create a git tag snapshot of chat sessions
    Snapshot {
        /// Project path
        path: String,

        /// Tag name (auto-generated if not provided)
        #[arg(long)]
        tag: Option<String>,

        /// Snapshot message
        #[arg(short, long)]
        message: Option<String>,
    },

    /// Track chat sessions together with associated file changes
    Track {
        /// Project path
        path: String,

        /// Commit message describing the changes
        #[arg(short, long)]
        message: Option<String>,

        /// Include all staged and unstaged file changes
        #[arg(long)]
        all: bool,

        /// Include specific files in addition to chat sessions
        #[arg(long)]
        files: Option<Vec<String>>,

        /// Create a tag for this tracked state
        #[arg(long)]
        tag: Option<String>,
    },

    /// Show history of chat session commits with associated file changes
    Log {
        /// Project path
        path: String,

        /// Number of commits to show
        #[arg(short = 'n', long, default_value = "10")]
        count: usize,

        /// Show only commits that include chat session changes
        #[arg(long)]
        sessions_only: bool,
    },

    /// Diff chat sessions between commits or current state
    Diff {
        /// Project path
        path: String,

        /// First commit (default: HEAD)
        #[arg(long)]
        from: Option<String>,

        /// Second commit (default: working directory)
        #[arg(long)]
        to: Option<String>,

        /// Show associated file changes alongside chat diffs
        #[arg(long)]
        with_files: bool,
    },

    /// Restore chat sessions from a specific commit
    Restore {
        /// Project path
        path: String,

        /// Commit hash, tag, or reference to restore from
        commit: String,

        /// Also restore associated files from the same commit
        #[arg(long)]
        with_files: bool,

        /// Create a backup before restoring
        #[arg(long)]
        backup: bool,
    },
}

// ============================================================================
// Migration Subcommands
// ============================================================================

#[derive(Subcommand)]
pub enum MigrationCommands {
    /// Create a migration package for moving to a new machine
    Create {
        /// Output directory for migration package
        output: String,

        /// Comma-separated list of project paths to include
        #[arg(long)]
        projects: Option<String>,

        /// Include all workspaces with chat sessions
        #[arg(long)]
        all: bool,
    },

    /// Restore a migration package on a new machine
    Restore {
        /// Path to migration package directory
        package: String,

        /// Project path mapping: 'old1:new1;old2:new2'
        #[arg(long)]
        mapping: Option<String>,

        /// Show what would be done without doing it
        #[arg(long)]
        dry_run: bool,
    },
}

// ============================================================================
// Run Subcommands
// ============================================================================

#[derive(Subcommand)]
pub enum RunCommands {
    /// Launch interactive TUI (Text User Interface)
    Tui,

    /// Run Ollama with automatic session recording
    Ollama {
        /// Model name (e.g., llama3, codellama, mistral)
        #[arg(short, long, default_value = "llama3")]
        model: String,

        /// Custom API endpoint
        #[arg(long)]
        endpoint: Option<String>,

        /// Workspace path for session context
        #[arg(long)]
        workspace: Option<String>,
    },

    /// Run Claude Code CLI with automatic session recording
    #[command(name = "claudecode")]
    ClaudeCode {
        /// Workspace path for session context
        #[arg(long)]
        workspace: Option<String>,
    },

    /// Run OpenCode with automatic session recording
    #[command(name = "opencode")]
    OpenCode {
        /// Workspace path for session context
        #[arg(long)]
        workspace: Option<String>,
    },

    /// Run Claude (Anthropic API) with automatic session recording
    Claude {
        /// Model name (e.g., claude-sonnet-4-20250514, claude-3-haiku)
        #[arg(short, long, default_value = "claude-sonnet-4-20250514")]
        model: String,

        /// Workspace path for session context
        #[arg(long)]
        workspace: Option<String>,
    },

    /// Run ChatGPT (OpenAI API) with automatic session recording
    #[command(name = "chatgpt")]
    ChatGpt {
        /// Model name (e.g., gpt-4o, gpt-4o-mini, o1)
        #[arg(short, long, default_value = "gpt-4o")]
        model: String,

        /// Workspace path for session context
        #[arg(long)]
        workspace: Option<String>,
    },
}

// ============================================================================
// Provider Subcommands
// ============================================================================

#[derive(Subcommand)]
pub enum ProviderCommands {
    /// List all discovered LLM providers
    List,

    /// Show detailed info about a specific provider
    Info {
        /// Provider name (copilot, cursor, ollama, vllm, foundry, lm-studio, etc.)
        provider: String,
    },

    /// Configure a provider
    Config {
        /// Provider name
        provider: String,

        /// API endpoint URL
        #[arg(long)]
        endpoint: Option<String>,

        /// API key
        #[arg(long)]
        api_key: Option<String>,

        /// Default model
        #[arg(long)]
        model: Option<String>,

        /// Enable or disable the provider
        #[arg(long)]
        enabled: Option<bool>,
    },

    /// Import sessions from another provider
    Import {
        /// Source provider name
        #[arg(long)]
        from: String,

        /// Target project path (or current directory)
        #[arg(long)]
        path: Option<String>,

        /// Session ID to import (omit for all)
        #[arg(long)]
        session: Option<String>,
    },

    /// Test connection to a provider
    Test {
        /// Provider name
        provider: String,
    },
}

// ============================================================================
// Detect Subcommands
// ============================================================================

#[derive(Subcommand)]
pub enum DetectCommands {
    /// Detect workspace for a path
    Workspace {
        /// Project path (default: current directory)
        path: Option<String>,
    },

    /// Detect available providers
    Providers {
        /// Only show providers with sessions
        #[arg(long)]
        with_sessions: bool,
    },

    /// Detect which provider a session belongs to
    Session {
        /// Session ID or filename
        session_id: String,

        /// Project path to search in
        #[arg(long)]
        path: Option<String>,
    },

    /// Detect everything (workspace, providers, sessions) for a path
    All {
        /// Project path (default: current directory)
        path: Option<String>,

        /// Show detailed information
        #[arg(long)]
        verbose: bool,
    },

    /// Find all workspace hashes for a project path (including orphaned workspaces with sessions)
    Orphaned {
        /// Project path (default: current directory)
        path: Option<String>,

        /// Automatically recover orphaned sessions by copying to active workspace
        #[arg(long, short)]
        recover: bool,
    },
}

// ============================================================================
// Register Subcommands
// ============================================================================

#[derive(Subcommand)]
pub enum RegisterCommands {
    /// Register all on-disk sessions into VS Code's index (fixes orphaned sessions)
    All {
        /// Project path (default: current directory)
        #[arg(long)]
        path: Option<String>,

        /// Merge all sessions into one before registering
        #[arg(long, short)]
        merge: bool,

        /// Force registration even if VS Code is running
        #[arg(long, short)]
        force: bool,
    },

    /// Register specific sessions by ID or title into VS Code's index
    #[command(visible_alias = "s")]
    Session {
        /// Session IDs or filenames (without .json extension)
        #[arg(required_unless_present = "title")]
        ids: Vec<String>,

        /// Match sessions by title instead of ID
        #[arg(long, short, num_args = 1.., value_delimiter = ' ')]
        title: Option<Vec<String>>,

        /// Project path (default: current directory)
        #[arg(long)]
        path: Option<String>,

        /// Force registration even if VS Code is running
        #[arg(long, short)]
        force: bool,
    },

    /// Recursively walk directories to find and register orphaned sessions for all workspaces
    #[command(visible_alias = "r")]
    Recursive {
        /// Root path to start recursive search (default: current directory)
        path: Option<String>,

        /// Maximum directory depth to recurse (default: unlimited)
        #[arg(long, short)]
        depth: Option<usize>,

        /// Force registration even if VS Code is running
        #[arg(long, short)]
        force: bool,

        /// Only show what would be registered without making changes
        #[arg(long)]
        dry_run: bool,

        /// Skip directories matching these patterns (can be used multiple times)
        #[arg(long, short = 'x')]
        exclude: Vec<String>,
    },
}

// ============================================================================
// Harvest Subcommands
// ============================================================================

#[derive(Subcommand)]
pub enum HarvestCommands {
    /// Initialize a harvest database
    Init {
        /// Path to the database file (default: ./chat_sessions.db)
        #[arg(long)]
        path: Option<String>,

        /// Initialize git tracking for the database
        #[arg(long)]
        git: bool,
    },

    /// Scan for available providers and sessions
    Scan {
        /// Show individual sessions
        #[arg(long)]
        sessions: bool,

        /// Scan for web-based LLM providers (ChatGPT, Claude, etc.)
        #[arg(long)]
        web: bool,

        /// Timeout in seconds for web provider checks (default: 5)
        #[arg(long, default_value = "5")]
        timeout: u64,

        /// Show verbose debug output for browser scanning
        #[arg(long, short)]
        verbose: bool,
    },

    /// Run the harvest to collect sessions from all providers
    Run {
        /// Path to the harvest database
        #[arg(long)]
        path: Option<String>,

        /// Only include specific providers (comma-separated: copilot,cursor,ollama)
        #[arg(long, value_delimiter = ',')]
        providers: Option<Vec<String>>,

        /// Exclude specific providers (comma-separated)
        #[arg(long, value_delimiter = ',')]
        exclude: Option<Vec<String>>,

        /// Only harvest sessions changed since last run
        #[arg(long)]
        incremental: bool,

        /// Auto-commit changes to git after harvest
        #[arg(long)]
        commit: bool,

        /// Commit message (requires --commit)
        #[arg(short, long)]
        message: Option<String>,
    },

    /// Show harvest database status
    Status {
        /// Path to the harvest database
        #[arg(long)]
        path: Option<String>,
    },

    /// List sessions in the harvest database
    List {
        /// Path to the harvest database
        #[arg(long)]
        path: Option<String>,

        /// Filter by provider name
        #[arg(long)]
        provider: Option<String>,

        /// Maximum number of sessions to show
        #[arg(long, default_value = "20")]
        limit: usize,

        /// Search sessions by title or ID
        #[arg(long)]
        search: Option<String>,
    },

    /// Export sessions from the harvest database
    Export {
        /// Output file path
        output: String,

        /// Path to the harvest database
        #[arg(long)]
        path: Option<String>,

        /// Export format: json, jsonl, md (markdown)
        #[arg(long, default_value = "json")]
        format: String,

        /// Filter by provider name
        #[arg(long)]
        provider: Option<String>,

        /// Export specific sessions by ID (comma-separated)
        #[arg(long, value_delimiter = ',')]
        sessions: Option<Vec<String>>,
    },

    /// Import a shared chat session from a URL
    Share {
        /// Share link URL (ChatGPT, Claude, etc.)
        url: String,

        /// Path to the harvest database
        #[arg(long)]
        path: Option<String>,

        /// Custom name for the imported session
        #[arg(long)]
        name: Option<String>,

        /// Associate with a workspace path
        #[arg(long)]
        workspace: Option<String>,
    },

    /// List pending or imported share links
    Shares {
        /// Path to the harvest database
        #[arg(long)]
        path: Option<String>,

        /// Filter by status: pending, imported, failed, expired
        #[arg(long)]
        status: Option<String>,

        /// Maximum number of links to show
        #[arg(long, default_value = "20")]
        limit: usize,
    },

    /// Create a checkpoint (version snapshot) of a session
    Checkpoint {
        /// Session ID to checkpoint
        session: String,

        /// Path to the harvest database
        #[arg(long)]
        path: Option<String>,

        /// Checkpoint description message
        #[arg(short, long)]
        message: Option<String>,
    },

    /// List checkpoints for a session
    Checkpoints {
        /// Session ID to list checkpoints for
        session: String,

        /// Path to the harvest database
        #[arg(long)]
        path: Option<String>,
    },

    /// Restore a session to a previous checkpoint
    Restore {
        /// Session ID to restore
        session: String,

        /// Checkpoint number to restore to
        checkpoint: i64,

        /// Path to the harvest database
        #[arg(long)]
        path: Option<String>,
    },

    /// Rebuild the full-text search index
    Rebuild {
        /// Path to the harvest database
        #[arg(long)]
        path: Option<String>,
    },

    /// Search messages across all sessions (full-text search)
    Search {
        /// Search query
        query: String,

        /// Path to the harvest database
        #[arg(long)]
        path: Option<String>,

        /// Filter by provider
        #[arg(long)]
        provider: Option<String>,

        /// Maximum results to show
        #[arg(long, default_value = "20")]
        limit: usize,
    },

    /// Git operations for the harvest database
    Git {
        #[command(subcommand)]
        command: HarvestGitCommands,
    },
}

#[derive(Subcommand)]
pub enum HarvestGitCommands {
    /// Initialize git tracking for the harvest database
    Init {
        /// Path to the harvest database
        #[arg(long)]
        path: Option<String>,
    },

    /// Commit changes to the harvest database
    Commit {
        /// Path to the harvest database
        #[arg(long)]
        path: Option<String>,

        /// Commit message
        #[arg(short, long)]
        message: Option<String>,
    },

    /// Show git log for the harvest database
    Log {
        /// Path to the harvest database
        #[arg(long)]
        path: Option<String>,

        /// Number of commits to show
        #[arg(short = 'n', long, default_value = "10")]
        count: usize,
    },

    /// Show changes to the harvest database
    Diff {
        /// Path to the harvest database
        #[arg(long)]
        path: Option<String>,

        /// Compare against specific commit
        #[arg(long)]
        commit: Option<String>,
    },

    /// Restore harvest database from a commit
    Restore {
        /// Commit hash to restore from
        commit: String,

        /// Path to the harvest database
        #[arg(long)]
        path: Option<String>,
    },
}

// ============================================================================
// Recover Subcommands
// ============================================================================

#[derive(Subcommand)]
pub enum RecoverCommands {
    /// Scan for recoverable sessions from various sources
    Scan {
        /// Provider to scan: vscode, cursor, all (default: all)
        #[arg(long, default_value = "all")]
        provider: String,

        /// Show detailed information about each session
        #[arg(short, long)]
        verbose: bool,

        /// Include sessions older than normal retention period
        #[arg(long)]
        include_old: bool,
    },

    /// Recover sessions from the recording API server
    Recording {
        /// Server URL (default: http://localhost:8787)
        #[arg(long, default_value = "http://localhost:8787")]
        server: String,

        /// Only recover specific session ID
        #[arg(long)]
        session: Option<String>,

        /// Output directory for recovered sessions
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Recover sessions from SQLite database backups
    Database {
        /// Path to the database backup file
        backup: String,

        /// Extract specific session by ID
        #[arg(long)]
        session: Option<String>,

        /// Output directory for recovered sessions
        #[arg(short, long)]
        output: Option<String>,

        /// Output format: json, jsonl, md (default: json)
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// Recover sessions from incomplete/corrupted JSONL files
    Jsonl {
        /// Path to the JSONL file to repair
        file: String,

        /// Output file for recovered sessions (default: same name with .recovered suffix)
        #[arg(short, long)]
        output: Option<String>,

        /// Attempt aggressive recovery (may produce incomplete sessions)
        #[arg(long)]
        aggressive: bool,
    },

    /// List sessions from VS Code's workspaceStorage that may be orphaned
    Orphans {
        /// Provider to check: vscode, cursor, all (default: all)
        #[arg(long, default_value = "all")]
        provider: String,

        /// Show sessions not in the SQLite state database
        #[arg(long)]
        unindexed: bool,

        /// Check if files actually exist on disk
        #[arg(long)]
        verify: bool,
    },

    /// Repair corrupted session files in place
    Repair {
        /// Path to the session directory or file
        path: String,

        /// Create backup before repair
        #[arg(long, default_value = "true")]
        backup: bool,

        /// Dry run - show what would be repaired without making changes
        #[arg(long)]
        dry_run: bool,
    },

    /// Show recovery status and recommendations
    Status {
        /// Provider to check: vscode, cursor, all (default: all)
        #[arg(long, default_value = "all")]
        provider: String,

        /// Check disk space and file system health
        #[arg(long)]
        system: bool,
    },

    /// Convert session files between JSON and JSONL formats
    Convert {
        /// Input file to convert (.json or .jsonl)
        input: String,

        /// Output file (auto-detects format from extension, or uses --format)
        #[arg(short, long)]
        output: Option<String>,

        /// Output format: json, jsonl (default: opposite of input)
        #[arg(long)]
        format: Option<String>,

        /// VS Code version compatibility: legacy (< 1.109), modern (>= 1.109), both
        #[arg(long, default_value = "both")]
        compat: String,
    },

    /// Extract sessions from a VS Code workspace by project path
    Extract {
        /// Project directory path (will find corresponding workspace hash)
        path: String,

        /// Output directory for extracted sessions
        #[arg(short, long)]
        output: Option<String>,

        /// Include both JSON and JSONL formats if available
        #[arg(long)]
        all_formats: bool,

        /// Include editing session fragments (agent mode work)
        #[arg(long)]
        include_edits: bool,
    },

    /// Detect and display session format and version information
    Detect {
        /// Session file to analyze (.json or .jsonl)
        file: String,

        /// Show raw format detection details
        #[arg(long)]
        verbose: bool,

        /// Output detection result as JSON
        #[arg(long)]
        json: bool,
    },
}

// ============================================================================
// API Server Subcommands
// ============================================================================

#[derive(Subcommand)]
pub enum ApiCommands {
    /// Start the API server
    Serve {
        /// Host to bind to (default: 0.0.0.0 for all interfaces)
        #[arg(long, default_value = "0.0.0.0")]
        host: String,

        /// Port to listen on (default: 8787)
        #[arg(short, long, default_value = "8787")]
        port: u16,

        /// Path to the database file
        #[arg(long)]
        database: Option<String>,
    },
}

// ============================================================================
// Agency (Agent Development Kit) Subcommands
// ============================================================================

#[derive(Subcommand)]
pub enum AgencyCommands {
    /// List available agents and their roles
    List {
        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,
    },

    /// Show agent information
    Info {
        /// Agent name or ID
        name: String,
    },

    /// List supported orchestration modes
    Modes,

    /// Run an agent with a prompt
    Run {
        /// Agent name to run
        #[arg(short, long, default_value = "assistant")]
        agent: String,

        /// Prompt or task for the agent
        prompt: String,

        /// Model to use (e.g., gemini-2.0-flash, gpt-4o)
        #[arg(short, long)]
        model: Option<String>,

        /// Orchestration mode (single, sequential, parallel, swarm)
        #[arg(long, default_value = "single")]
        orchestration: String,

        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Create a new agent configuration
    Create {
        /// Agent name
        name: String,

        /// Agent role (coordinator, researcher, coder, reviewer, executor, writer, tester, custom)
        #[arg(short, long, default_value = "custom")]
        role: String,

        /// System instruction for the agent
        #[arg(short, long)]
        instruction: Option<String>,

        /// Model to use
        #[arg(short, long)]
        model: Option<String>,
    },

    /// List available tools
    Tools,

    /// Show swarm templates
    Templates,
}

// ============================================================================
// Telemetry Subcommands
// ============================================================================

#[derive(Subcommand)]
pub enum TelemetryCommands {
    /// Show telemetry status and what data is collected
    #[command(visible_alias = "status")]
    Info,

    /// Enable anonymous usage data collection (this is the default)
    #[command(visible_alias = "enable")]
    OptIn,

    /// Disable anonymous usage data collection
    #[command(visible_alias = "disable")]
    OptOut,

    /// Reset telemetry ID (generates new anonymous identifier)
    Reset,

    /// Record structured data for later AI analysis
    #[command(visible_alias = "log")]
    Record {
        /// Event category (e.g., 'workflow', 'error', 'performance', 'usage')
        #[arg(short, long, default_value = "custom")]
        category: String,

        /// Event name or type
        #[arg(short, long)]
        event: String,

        /// JSON data payload (or use --kv for key=value pairs)
        #[arg(short, long)]
        data: Option<String>,

        /// Key-value pairs (can be repeated: -k foo=bar -k baz=123)
        #[arg(short = 'k', long = "kv", value_parser = parse_key_value)]
        kv: Vec<(String, String)>,

        /// Add tags for filtering (can be repeated: -t important -t session-123)
        #[arg(short, long)]
        tags: Vec<String>,

        /// Optional session or context ID to associate with
        #[arg(long)]
        context: Option<String>,

        /// Print recorded event details
        #[arg(short, long)]
        verbose: bool,
    },

    /// Show recorded telemetry data
    #[command(visible_alias = "logs")]
    Show {
        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,

        /// Filter by event name
        #[arg(short, long)]
        event: Option<String>,

        /// Filter by tag
        #[arg(short, long)]
        tag: Option<String>,

        /// Maximum number of records to show
        #[arg(short = 'n', long, default_value = "20")]
        limit: usize,

        /// Output format: table, json, jsonl
        #[arg(short, long, default_value = "table")]
        format: String,

        /// Show records after this date (YYYY-MM-DD)
        #[arg(long)]
        after: Option<String>,

        /// Show records before this date (YYYY-MM-DD)
        #[arg(long)]
        before: Option<String>,
    },

    /// Export recorded data for AI analysis
    Export {
        /// Output file path
        output: String,

        /// Export format: json, jsonl, csv
        #[arg(short, long, default_value = "jsonl")]
        format: String,

        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,

        /// Include installation metadata in export
        #[arg(long)]
        with_metadata: bool,
    },

    /// Clear recorded telemetry data
    Clear {
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,

        /// Only clear records older than N days
        #[arg(long)]
        older_than: Option<u32>,
    },

    /// Configure remote telemetry endpoint
    Config {
        /// Set the remote endpoint URL
        #[arg(long)]
        endpoint: Option<String>,

        /// Set the API key for authentication
        #[arg(long)]
        api_key: Option<String>,

        /// Enable remote telemetry sending
        #[arg(long)]
        enable_remote: bool,

        /// Disable remote telemetry sending
        #[arg(long)]
        disable_remote: bool,
    },

    /// Sync telemetry records to remote server
    Sync {
        /// Maximum number of records to sync
        #[arg(short = 'n', long)]
        limit: Option<usize>,

        /// Clear local records after successful sync
        #[arg(long)]
        clear_after: bool,
    },

    /// Test connection to remote telemetry server
    Test,
}

/// Parse key=value pairs for telemetry record command
fn parse_key_value(s: &str) -> std::result::Result<(String, String), String> {
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid key=value pair: no '=' found in '{s}'"))?;
    Ok((s[..pos].to_string(), s[pos + 1..].to_string()))
}
