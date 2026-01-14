// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Chat System Manager (csm) - Main entry point
//!
//! A CLI tool to manage and merge chat sessions across workspaces.

#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::type_complexity)]

mod agency;
mod api;
mod browser;
mod cli;
mod commands;
mod database;
mod error;
mod mcp;
mod models;
mod providers;
mod storage;
mod tui;
mod workspace;

use anyhow::Result;
use clap::Parser;
use cli::{
    AgencyCommands, ApiCommands, Cli, Commands, DetectCommands, ExportCommands, FetchCommands,
    FindCommands, GitCommands, HarvestCommands, HarvestGitCommands, ImportCommands, ListCommands,
    MergeCommands, MigrationCommands, MoveCommands, ProviderCommands, RunCommands, ShowCommands,
};

/// Get the current directory name as a default pattern
fn get_current_dir_name() -> String {
    std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| ".".to_string())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        // ====================================================================
        // List Commands
        // ====================================================================
        Commands::List { command } => match command {
            Some(ListCommands::Workspaces) => commands::list_workspaces(),
            Some(ListCommands::Sessions { project_path }) => {
                commands::list_sessions(project_path.as_deref())
            }
            Some(ListCommands::Path { project_path }) => {
                commands::list_sessions(project_path.as_deref())
            }
            Some(ListCommands::Orphaned { path }) => commands::list_orphaned(path.as_deref()),
            Some(ListCommands::Providers { with_sessions }) => {
                commands::detect_providers(with_sessions)
            }
            None => commands::list_workspaces(), // Default to workspaces
        },

        // ====================================================================
        // Find Commands
        // ====================================================================
        Commands::Find { command } => match command {
            Some(FindCommands::Workspace { pattern }) => {
                let pattern = pattern.unwrap_or_else(get_current_dir_name);
                commands::find_workspaces(&pattern)
            }
            Some(FindCommands::Session {
                pattern,
                workspace,
                title_only,
                content,
                after,
                before,
                limit,
            }) => {
                let pattern = pattern.unwrap_or_else(get_current_dir_name);
                commands::find_sessions_filtered(
                    &pattern,
                    workspace.as_deref(),
                    title_only,
                    content,
                    after.as_deref(),
                    before.as_deref(),
                    limit,
                )
            }
            Some(FindCommands::Path {
                pattern,
                project_path,
            }) => {
                let pattern = pattern.unwrap_or_else(get_current_dir_name);
                // Use title-only search by default for path-based search (faster)
                commands::find_sessions_filtered(
                    &pattern,
                    project_path.as_deref(),
                    false,
                    false,
                    None,
                    None,
                    50,
                )
            }
            None => {
                // Default to finding workspaces matching current directory
                let pattern = get_current_dir_name();
                commands::find_workspaces(&pattern)
            }
        },

        // ====================================================================
        // Show Commands
        // ====================================================================
        Commands::Show { command } => match command {
            Some(ShowCommands::Workspace { workspace }) => commands::show_workspace(&workspace),
            Some(ShowCommands::Session {
                session_id,
                project_path,
            }) => commands::show_session(&session_id, project_path.as_deref()),
            Some(ShowCommands::Path { project_path }) => {
                commands::history_show(project_path.as_deref())
            }
            None => commands::history_show(None), // Default to current directory
        },

        // ====================================================================
        // Fetch Commands
        // ====================================================================
        Commands::Fetch { command } => match command {
            Some(FetchCommands::Workspace {
                workspace_name,
                target_path,
                force,
                no_register,
            }) => commands::fetch_by_workspace(
                &workspace_name,
                target_path.as_deref(),
                force,
                no_register,
            ),
            Some(FetchCommands::Session {
                session_ids,
                target_path,
                force,
                no_register,
            }) => {
                commands::fetch_sessions(&session_ids, target_path.as_deref(), force, no_register)
            }
            Some(FetchCommands::Path {
                project_path,
                force,
                no_register,
            }) => commands::history_fetch(project_path.as_deref(), force, no_register),
            None => {
                eprintln!("Usage: csm fetch <workspace|session|path> ...");
                eprintln!("Run 'csm fetch --help' for more information.");
                Ok(())
            }
        },

        // ====================================================================
        // Merge Commands
        // ====================================================================
        Commands::Merge { command } => match command {
            Some(MergeCommands::Workspace {
                workspace_name,
                title,
                target_path,
                force,
                no_backup,
            }) => commands::merge_by_workspace_name(
                &workspace_name,
                title.as_deref(),
                target_path.as_deref(),
                force,
                no_backup,
            ),
            Some(MergeCommands::Workspaces {
                workspace_names,
                title,
                target_path,
                force,
                no_backup,
            }) => commands::merge_by_workspace_names(
                &workspace_names,
                title.as_deref(),
                target_path.as_deref(),
                force,
                no_backup,
            ),
            Some(MergeCommands::Sessions {
                sessions,
                title,
                target_path,
                force,
                no_backup,
            }) => commands::merge_sessions_by_list(
                &sessions,
                title.as_deref(),
                target_path.as_deref(),
                force,
                no_backup,
            ),
            Some(MergeCommands::Path {
                project_path,
                title,
                force,
                no_backup,
            }) => {
                commands::history_merge(project_path.as_deref(), title.as_deref(), force, no_backup)
            }
            Some(MergeCommands::Provider {
                provider_name,
                title,
                target_path,
                sessions,
                force,
                no_backup,
            }) => commands::merge_from_provider(
                &provider_name,
                title.as_deref(),
                target_path.as_deref(),
                sessions.as_deref(),
                force,
                no_backup,
            ),
            Some(MergeCommands::Providers {
                providers,
                title,
                target_path,
                workspace,
                force,
                no_backup,
            }) => commands::merge_cross_provider(
                &providers,
                title.as_deref(),
                target_path.as_deref(),
                workspace.as_deref(),
                force,
                no_backup,
            ),
            Some(MergeCommands::All {
                title,
                target_path,
                workspace,
                force,
                no_backup,
            }) => commands::merge_all_providers(
                title.as_deref(),
                target_path.as_deref(),
                workspace.as_deref(),
                force,
                no_backup,
            ),
            None => {
                eprintln!("Usage: csm merge <workspace|workspaces|sessions|path|provider|providers|all> ...");
                eprintln!("Run 'csm merge --help' for more information.");
                Ok(())
            }
        },

        // ====================================================================
        // Export Commands
        // ====================================================================
        Commands::Export { command } => match command {
            Some(ExportCommands::Workspace { destination, hash }) => {
                commands::export_sessions(&destination, Some(&hash), None)
            }
            Some(ExportCommands::Sessions {
                destination,
                session_ids,
                project_path,
            }) => commands::export_specific_sessions(
                &destination,
                &session_ids,
                project_path.as_deref(),
            ),
            Some(ExportCommands::Path {
                destination,
                project_path,
            }) => commands::export_sessions(&destination, None, project_path.as_deref()),
            None => {
                eprintln!("Usage: csm export <workspace|sessions|path> ...");
                eprintln!("Run 'csm export --help' for more information.");
                Ok(())
            }
        },

        // ====================================================================
        // Import Commands
        // ====================================================================
        Commands::Import { command } => match command {
            Some(ImportCommands::Workspace {
                source,
                hash,
                force,
            }) => commands::import_sessions(&source, Some(&hash), None, force),
            Some(ImportCommands::Sessions {
                session_files,
                target_path,
                force,
            }) => commands::import_specific_sessions(&session_files, target_path.as_deref(), force),
            Some(ImportCommands::Path {
                source,
                target_path,
                force,
            }) => commands::import_sessions(&source, None, target_path.as_deref(), force),
            None => {
                eprintln!("Usage: csm import <workspace|sessions|path> ...");
                eprintln!("Run 'csm import --help' for more information.");
                Ok(())
            }
        },

        // ====================================================================
        // Move Commands
        // ====================================================================
        Commands::Move { command } => match command {
            Some(MoveCommands::Workspace {
                source_hash,
                target,
            }) => commands::move_workspace(&source_hash, &target),
            Some(MoveCommands::Sessions {
                session_ids,
                target_path,
            }) => commands::move_specific_sessions(&session_ids, &target_path),
            Some(MoveCommands::Path {
                source_path,
                target_path,
            }) => commands::move_by_path(&source_path, &target_path),
            None => {
                eprintln!("Usage: csm move <workspace|sessions|path> ...");
                eprintln!("Run 'csm move --help' for more information.");
                Ok(())
            }
        },

        // ====================================================================
        // Git Commands
        // ====================================================================
        Commands::Git { command } => match command {
            GitCommands::Config { name, email, path } => {
                commands::git_config(name.as_deref(), email.as_deref(), path.as_deref())
            }
            GitCommands::Init { path } => commands::git_init(&path),
            GitCommands::Add {
                path,
                commit,
                message,
            } => commands::git_add(&path, commit, message.as_deref()),
            GitCommands::Status { path } => commands::git_status(&path),
            GitCommands::Snapshot { path, tag, message } => {
                commands::git_snapshot(&path, tag.as_deref(), message.as_deref())
            }
            GitCommands::Track {
                path,
                message,
                all,
                files,
                tag,
            } => commands::git_track(
                &path,
                message.as_deref(),
                all,
                files.as_deref(),
                tag.as_deref(),
            ),
            GitCommands::Log {
                path,
                count,
                sessions_only,
            } => commands::git_log(&path, count, sessions_only),
            GitCommands::Diff {
                path,
                from,
                to,
                with_files,
            } => commands::git_diff(&path, from.as_deref(), to.as_deref(), with_files),
            GitCommands::Restore {
                path,
                commit,
                with_files,
                backup,
            } => commands::git_restore(&path, &commit, with_files, backup),
        },

        // ====================================================================
        // Migration Commands
        // ====================================================================
        Commands::Migration { command } => match command {
            MigrationCommands::Create {
                output,
                projects,
                all,
            } => commands::create_migration(&output, projects.as_deref(), all),
            MigrationCommands::Restore {
                package,
                mapping,
                dry_run,
            } => commands::restore_migration(&package, mapping.as_deref(), dry_run),
        },

        // ====================================================================
        // Run Commands (TUI)
        // ====================================================================
        Commands::Run { command } => match command {
            RunCommands::Tui => tui::run_tui(),
        },

        // ====================================================================
        // Provider Commands
        // ====================================================================
        Commands::Provider { command } => match command {
            ProviderCommands::List => commands::list_providers(),
            ProviderCommands::Info { provider } => commands::provider_info(&provider),
            ProviderCommands::Config {
                provider,
                endpoint,
                api_key,
                model,
                enabled,
            } => commands::configure_provider(
                &provider,
                endpoint.as_deref(),
                api_key.as_deref(),
                model.as_deref(),
                enabled,
            ),
            ProviderCommands::Import {
                from,
                path,
                session,
            } => commands::import_from_provider(&from, path.as_deref(), session.as_deref()),
            ProviderCommands::Test { provider } => commands::test_provider(&provider),
        },

        // ====================================================================
        // Detect Commands
        // ====================================================================
        Commands::Detect { command } => match command {
            Some(DetectCommands::Workspace { path }) => commands::detect_workspace(path.as_deref()),
            Some(DetectCommands::Providers { with_sessions }) => {
                commands::detect_providers(with_sessions)
            }
            Some(DetectCommands::Session { session_id, path }) => {
                commands::detect_session(&session_id, path.as_deref())
            }
            Some(DetectCommands::All { path, verbose }) => {
                commands::detect_all(path.as_deref(), verbose)
            }
            None => {
                // Default to detect all for current directory
                commands::detect_all(None, false)
            }
        },

        // ====================================================================
        // Harvest Commands
        // ====================================================================
        Commands::Harvest { command } => match command {
            HarvestCommands::Init { path, git } => commands::harvest_init(path.as_deref(), git),
            HarvestCommands::Scan {
                sessions,
                web,
                timeout,
                verbose,
            } => commands::harvest_scan(sessions, web, timeout, verbose),
            HarvestCommands::Run {
                path,
                providers,
                exclude,
                incremental,
                commit,
                message,
            } => commands::harvest_run(
                path.as_deref(),
                providers.as_deref(),
                exclude.as_deref(),
                incremental,
                commit,
                message.as_deref(),
            ),
            HarvestCommands::Status { path } => commands::harvest_status(path.as_deref()),
            HarvestCommands::List {
                path,
                provider,
                limit,
                search,
            } => commands::harvest_list(
                path.as_deref(),
                provider.as_deref(),
                limit,
                search.as_deref(),
            ),
            HarvestCommands::Export {
                output,
                path,
                format,
                provider,
                sessions,
            } => commands::harvest_export(
                path.as_deref(),
                &output,
                &format,
                provider.as_deref(),
                sessions.as_deref(),
            ),
            HarvestCommands::Share {
                url,
                path,
                name,
                workspace,
            } => commands::harvest_share(
                path.as_deref(),
                &url,
                name.as_deref(),
                workspace.as_deref(),
            ),
            HarvestCommands::Shares {
                path,
                status,
                limit,
            } => commands::harvest_shares(path.as_deref(), status.as_deref(), limit),
            HarvestCommands::Checkpoint {
                session,
                path,
                message,
            } => commands::harvest_checkpoint(path.as_deref(), &session, message.as_deref()),
            HarvestCommands::Checkpoints { session, path } => {
                commands::harvest_checkpoints(path.as_deref(), &session)
            }
            HarvestCommands::Restore {
                session,
                checkpoint,
                path,
            } => commands::harvest_restore_checkpoint(path.as_deref(), &session, checkpoint),
            HarvestCommands::Rebuild { path } => commands::harvest_rebuild_fts(path.as_deref()),
            HarvestCommands::Search {
                query,
                path,
                provider,
                limit,
            } => commands::harvest_search(path.as_deref(), &query, provider.as_deref(), limit),
            HarvestCommands::Git { command: git_cmd } => match git_cmd {
                HarvestGitCommands::Init { path } => commands::harvest_git_init(path.as_deref()),
                HarvestGitCommands::Commit { path, message } => {
                    commands::harvest_git_commit(path.as_deref(), message.as_deref())
                }
                HarvestGitCommands::Log { path, count } => {
                    commands::harvest_git_log(path.as_deref(), count)
                }
                HarvestGitCommands::Diff { path, commit } => {
                    commands::harvest_git_diff(path.as_deref(), commit.as_deref())
                }
                HarvestGitCommands::Restore { commit, path } => {
                    commands::harvest_git_restore(path.as_deref(), &commit)
                }
            },
        },

        // ====================================================================
        // Register Commands
        // ====================================================================
        Commands::Register { command } => match command {
            cli::RegisterCommands::All { path, merge, force } => {
                commands::register_all(path.as_deref(), merge, force)
            }
            cli::RegisterCommands::Session {
                ids,
                title,
                path,
                force,
            } => commands::register_sessions(&ids, title.as_deref(), path.as_deref(), force),
        },

        // ====================================================================
        // API Server
        // ====================================================================
        Commands::Api { command } => match command {
            ApiCommands::Serve {
                host,
                port,
                database,
            } => {
                let config = api::ServerConfig {
                    host,
                    port,
                    database_path: database.unwrap_or_else(|| {
                        dirs::data_local_dir()
                            .map(|p| p.join("csm").join("csm.db").to_string_lossy().to_string())
                            .unwrap_or_else(|| "csm.db".to_string())
                    }),
                    ..Default::default()
                };

                // Create tokio runtime and run the server
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()?;
                rt.block_on(api::start_server(config))
            }
        },

        // ====================================================================
        // Agency (Agent Development Kit)
        // ====================================================================
        Commands::Agency { command } => match command {
            AgencyCommands::List { verbose } => commands::list_agents(verbose),
            AgencyCommands::Info { name } => commands::show_agent_info(&name),
            AgencyCommands::Modes => commands::list_modes(),
            AgencyCommands::Run {
                agent,
                prompt,
                model,
                orchestration,
                verbose,
            } => commands::run_agent(&agent, &prompt, model.as_deref(), &orchestration, verbose),
            AgencyCommands::Create {
                name,
                role,
                instruction,
                model,
            } => commands::create_agent(&name, &role, instruction.as_deref(), model.as_deref()),
            AgencyCommands::Tools => commands::list_tools(),
            AgencyCommands::Templates => commands::list_templates(),
        },

        // ====================================================================
        // Easter Egg
        // ====================================================================
        Commands::Banner => {
            print_banner();
            Ok(())
        }
    }
}

fn print_banner() {
    use colored::Colorize;

    let banner = r#"
     .d8888b.  888    888        d8888  .d8888b.  888b     d888
    d88P  Y88b 888    888       d88888 d88P  Y88b 8888b   d8888
    888    888 888    888      d88P888 Y88b.      88888b.d88888
    888        8888888888     d88P 888  "Y888b.   888Y88888P888
    888        888    888    d88P  888     "Y88b. 888 Y888P 888
    888    888 888    888   d88P   888       "888 888  Y8P  888
    Y88b  d88P 888    888  d8888888888 Y88b  d88P 888   "   888
     "Y8888P"  888    888 d88P     888  "Y8888P"  888       888
    "#;

    let subtitle = "CHAt System Manager (Chasm) for Bridging LLM Providers";
    let tagline = "     Your AI providers and chat sessions, unified";
    let version = format!("                       v{}", env!("CARGO_PKG_VERSION"));

    println!("{}", banner.cyan().bold());
    println!("{}", subtitle.white().bold());
    println!("{}", tagline.bright_black());
    println!("{}", version.bright_black());
    println!();

    // Random fun messages
    let messages = [
        "[*] Managing your AI memories since 2024",
        "[*] Where conversations never get lost",
        "[*] Because context switching shouldn't mean losing context",
        "[*] Unifying the chaos of multi-LLM life",
        "[*] One tool to find them all",
        "[*] Faster than scrolling through old chats",
        "[*] From VS Code to the cloud and back",
        "[*] Built with Rust, powered by caffeine",
        "[*] Bridging the chasm between your chat sessions",
    ];

    let idx = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as usize % messages.len())
        .unwrap_or(0);

    println!("    {}", messages[idx].bright_yellow());
    println!();
}
