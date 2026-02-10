// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Git integration commands

use anyhow::{Context, Result};
use colored::*;
use std::path::Path;
use std::process::Command;

use crate::workspace::get_workspace_by_path;

/// Configure git settings for chat sessions
pub fn git_config(name: Option<&str>, email: Option<&str>, path: Option<&str>) -> Result<()> {
    let project_dir = path.map(Path::new).unwrap_or_else(|| Path::new("."));

    // Check if git repo exists
    if !project_dir.join(".git").exists() {
        anyhow::bail!("Not a git repository: {}", project_dir.display());
    }

    // If no options provided, show current config
    if name.is_none() && email.is_none() {
        println!("Git configuration for: {}", project_dir.display());

        let output = Command::new("git")
            .current_dir(project_dir)
            .args(["config", "--local", "user.name"])
            .output()?;
        let current_name = String::from_utf8_lossy(&output.stdout).trim().to_string();

        let output = Command::new("git")
            .current_dir(project_dir)
            .args(["config", "--local", "user.email"])
            .output()?;
        let current_email = String::from_utf8_lossy(&output.stdout).trim().to_string();

        println!(
            "  Name:  {}",
            if current_name.is_empty() {
                "(not set)".to_string()
            } else {
                current_name
            }
        );
        println!(
            "  Email: {}",
            if current_email.is_empty() {
                "(not set)".to_string()
            } else {
                current_email
            }
        );
        return Ok(());
    }

    // Set name if provided
    if let Some(n) = name {
        let output = Command::new("git")
            .current_dir(project_dir)
            .args(["config", "--local", "user.name", n])
            .output()?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to set git user.name: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        println!("{} Set git user.name = {}", "[OK]".green(), n);
    }

    // Set email if provided
    if let Some(e) = email {
        let output = Command::new("git")
            .current_dir(project_dir)
            .args(["config", "--local", "user.email", e])
            .output()?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to set git user.email: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        println!("{} Set git user.email = {}", "[OK]".green(), e);
    }

    Ok(())
}

/// Initialize git versioning for chat sessions
pub fn git_init(project_path: &str) -> Result<()> {
    let workspace = get_workspace_by_path(project_path)?
        .context(format!("Workspace not found for path: {}", project_path))?;

    let project_dir = Path::new(project_path);
    let vscode_dir = project_dir.join(".vscode");
    let symlink_path = vscode_dir.join("chat-sessions");

    // Create .vscode directory if needed
    std::fs::create_dir_all(&vscode_dir)?;

    // Create symlink to chat sessions
    if symlink_path.exists() {
        println!("{} Chat versioning already initialized", "[!]".yellow());
        println!("   Symlink: {}", symlink_path.display());
        return Ok(());
    }

    #[cfg(unix)]
    std::os::unix::fs::symlink(&workspace.chat_sessions_path, &symlink_path)?;

    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&workspace.chat_sessions_path, &symlink_path)?;

    println!(
        "{} Initialized git versioning for chat sessions",
        "[OK]".green()
    );
    println!("   Symlink: {}", symlink_path.display());
    println!("   Target: {}", workspace.chat_sessions_path.display());
    println!("\nNext steps:");
    println!("  1. Add .vscode/chat-sessions to your .gitignore if you want to exclude them");
    println!(
        "  2. Or commit them: csm add {} --commit -m 'Add chat sessions'",
        project_path
    );

    Ok(())
}

/// Add chat sessions to git
pub fn git_add(project_path: &str, commit: bool, message: Option<&str>) -> Result<()> {
    let project_dir = Path::new(project_path);
    let chat_sessions_path = project_dir.join(".vscode").join("chat-sessions");

    if !chat_sessions_path.exists() {
        anyhow::bail!(
            "Chat versioning not initialized. Run 'csm init {}' first",
            project_path
        );
    }

    // Stage files
    let output = Command::new("git")
        .current_dir(project_dir)
        .args(["add", ".vscode/chat-sessions"])
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to stage chat sessions: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    println!("{} Staged chat sessions for commit", "[OK]".green());

    // Commit if requested
    if commit {
        let msg = message.unwrap_or("Update chat sessions");

        let output = Command::new("git")
            .current_dir(project_dir)
            .args(["commit", "-m", msg])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("nothing to commit") {
                println!("{} Nothing to commit", "[i]".blue());
            } else {
                anyhow::bail!("Failed to commit: {}", stderr);
            }
        } else {
            // Get commit hash
            let output = Command::new("git")
                .current_dir(project_dir)
                .args(["rev-parse", "--short", "HEAD"])
                .output()?;

            let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("{} Committed: {}", "[OK]".green(), hash);
        }
    }

    Ok(())
}

/// Show git status of chat sessions
pub fn git_status(project_path: &str) -> Result<()> {
    let project_dir = Path::new(project_path);

    // Check if it's a git repo
    let is_git_repo = project_dir.join(".git").exists();

    // Check if versioning is enabled
    let chat_sessions_path = project_dir.join(".vscode").join("chat-sessions");
    let versioning_enabled = chat_sessions_path.exists();

    // Get workspace info
    let workspace = get_workspace_by_path(project_path)?;
    let session_count = workspace.map(|w| w.chat_session_count).unwrap_or(0);

    println!("Project: {}", project_path);
    println!("Git repository: {}", if is_git_repo { "Yes" } else { "No" });
    println!(
        "Chat versioning: {}",
        if versioning_enabled {
            "Enabled"
        } else {
            "Disabled"
        }
    );
    println!("Total sessions: {}", session_count);

    if versioning_enabled && is_git_repo {
        // Get git status for chat sessions
        let output = Command::new("git")
            .current_dir(project_dir)
            .args(["status", "--porcelain", ".vscode/chat-sessions"])
            .output()?;

        let status = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = status.lines().collect();

        let modified: Vec<_> = lines
            .iter()
            .filter(|l| l.starts_with(" M") || l.starts_with("M "))
            .collect();
        let untracked: Vec<_> = lines.iter().filter(|l| l.starts_with("??")).collect();
        let staged: Vec<_> = lines
            .iter()
            .filter(|l| l.starts_with("A ") || l.starts_with("M "))
            .collect();

        println!("\nGit status:");
        println!("  Modified: {}", modified.len());
        println!("  Untracked: {}", untracked.len());
        println!("  Staged: {}", staged.len());

        if !modified.is_empty() {
            println!("\n  Modified files:");
            for f in modified.iter().take(5) {
                println!(
                    "    - {}",
                    f.trim_start_matches(|c: char| c.is_whitespace() || c == 'M')
                );
            }
            if modified.len() > 5 {
                println!("    ... and {} more", modified.len() - 5);
            }
        }
    }

    Ok(())
}

/// Create a git tag snapshot of chat sessions
pub fn git_snapshot(project_path: &str, tag: Option<&str>, message: Option<&str>) -> Result<()> {
    let project_dir = Path::new(project_path);
    let chat_sessions_path = project_dir.join(".vscode").join("chat-sessions");

    if !chat_sessions_path.exists() {
        anyhow::bail!(
            "Chat versioning not initialized. Run 'csm init {}' first",
            project_path
        );
    }

    // Generate tag name if not provided
    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let tag_name = tag
        .map(|t| t.to_string())
        .unwrap_or_else(|| format!("chat-snapshot-{}", timestamp));

    let msg = message.unwrap_or("Chat session snapshot");

    // Stage and commit
    let _ = Command::new("git")
        .current_dir(project_dir)
        .args(["add", ".vscode/chat-sessions"])
        .output()?;

    let _ = Command::new("git")
        .current_dir(project_dir)
        .args(["commit", "-m", &format!("Snapshot: {}", msg)])
        .output()?;

    // Create tag
    let output = Command::new("git")
        .current_dir(project_dir)
        .args(["tag", "-a", &tag_name, "-m", msg])
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to create tag: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Get commit hash
    let output = Command::new("git")
        .current_dir(project_dir)
        .args(["rev-parse", "--short", "HEAD"])
        .output()?;

    let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();

    println!("{} Created snapshot", "[OK]".green());
    println!("   Tag: {}", tag_name);
    println!("   Commit: {}", hash);

    Ok(())
}

/// Track chat sessions together with associated file changes
pub fn git_track(
    project_path: &str,
    message: Option<&str>,
    all: bool,
    files: Option<&[String]>,
    tag: Option<&str>,
) -> Result<()> {
    let project_dir = Path::new(project_path);
    let chat_sessions_path = project_dir.join(".vscode").join("chat-sessions");

    if !chat_sessions_path.exists() {
        anyhow::bail!(
            "Chat versioning not initialized. Run 'csm git init {}' first",
            project_path
        );
    }

    println!(
        "{} Tracking chat sessions with file changes",
        "[*]".blue().bold()
    );
    println!("{}", "=".repeat(60));

    // Stage chat sessions
    let output = Command::new("git")
        .current_dir(project_dir)
        .args(["add", ".vscode/chat-sessions"])
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to stage chat sessions: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    println!("{} Staged chat sessions", "[OK]".green());

    // Stage additional files if requested
    if all {
        let output = Command::new("git")
            .current_dir(project_dir)
            .args(["add", "-A"])
            .output()?;

        if output.status.success() {
            println!("{} Staged all changes", "[OK]".green());
        }
    } else if let Some(file_list) = files {
        for file in file_list {
            let output = Command::new("git")
                .current_dir(project_dir)
                .args(["add", file])
                .output()?;

            if output.status.success() {
                println!("{} Staged: {}", "[OK]".green(), file);
            } else {
                println!("{} Failed to stage: {}", "[!]".yellow(), file);
            }
        }
    }

    // Get status summary
    let output = Command::new("git")
        .current_dir(project_dir)
        .args(["diff", "--cached", "--stat"])
        .output()?;

    let stat = String::from_utf8_lossy(&output.stdout);
    if !stat.is_empty() {
        println!("\n{} Changes to be committed:", "[*]".blue());
        for line in stat.lines().take(10) {
            println!("   {}", line);
        }
        if stat.lines().count() > 10 {
            println!("   ... and {} more files", stat.lines().count() - 10);
        }
    }

    // Generate commit message
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string();
    let default_msg = format!("Track chat sessions with changes ({})", timestamp);
    let commit_msg = message.unwrap_or(&default_msg);

    // Create commit
    let output = Command::new("git")
        .current_dir(project_dir)
        .args(["commit", "-m", commit_msg])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("nothing to commit") {
            println!("\n{} Nothing to commit", "[i]".blue());
            return Ok(());
        }
        anyhow::bail!("Failed to commit: {}", stderr);
    }

    // Get commit hash
    let output = Command::new("git")
        .current_dir(project_dir)
        .args(["rev-parse", "--short", "HEAD"])
        .output()?;

    let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
    println!("\n{} Committed: {}", "[OK]".green(), hash);

    // Create tag if requested
    if let Some(tag_name) = tag {
        let output = Command::new("git")
            .current_dir(project_dir)
            .args(["tag", "-a", tag_name, "-m", commit_msg])
            .output()?;

        if output.status.success() {
            println!("{} Tagged: {}", "[OK]".green(), tag_name);
        }
    }

    Ok(())
}

/// Show history of chat session commits with associated file changes
pub fn git_log(project_path: &str, count: usize, sessions_only: bool) -> Result<()> {
    let project_dir = Path::new(project_path);

    println!("{} Chat Session History", "[*]".blue().bold());
    println!("{}", "=".repeat(60));

    // Build log command
    let mut args = vec![
        "log".to_string(),
        format!("-{}", count),
        "--pretty=format:%h|%ad|%s".to_string(),
        "--date=short".to_string(),
    ];

    if sessions_only {
        args.push("--".to_string());
        args.push(".vscode/chat-sessions".to_string());
    }

    let output = Command::new("git")
        .current_dir(project_dir)
        .args(&args)
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to get git log: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let log = String::from_utf8_lossy(&output.stdout);

    if log.is_empty() {
        println!("\n{} No commits found", "[i]".blue());
        return Ok(());
    }

    println!();
    for line in log.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 3 {
            let hash = parts[0];
            let date = parts[1];
            let message = parts[2];

            // Check if this commit touched chat sessions
            let output = Command::new("git")
                .current_dir(project_dir)
                .args(["diff-tree", "--no-commit-id", "--name-only", "-r", hash])
                .output()?;

            let files = String::from_utf8_lossy(&output.stdout);
            let has_chat = files.contains("chat-sessions");
            let file_count = files.lines().count();

            let chat_marker = if has_chat {
                "[chat]".cyan()
            } else {
                "      ".normal()
            };

            println!(
                "{} {} {} {} ({})",
                hash.yellow(),
                chat_marker,
                date.dimmed(),
                message,
                format!("{} files", file_count).dimmed()
            );
        }
    }

    println!();
    println!(
        "{} Use 'csm git diff --from <hash>' to see changes",
        "[i]".cyan()
    );

    Ok(())
}

/// Diff chat sessions between commits or current state
pub fn git_diff(
    project_path: &str,
    from: Option<&str>,
    to: Option<&str>,
    with_files: bool,
) -> Result<()> {
    let project_dir = Path::new(project_path);

    let from_ref = from.unwrap_or("HEAD");
    let to_ref = to.unwrap_or("");

    println!("{} Chat Session Diff", "[*]".blue().bold());
    println!("{}", "=".repeat(60));

    if to_ref.is_empty() {
        println!("{} {}..working directory", "[>]".blue(), from_ref);
    } else {
        println!("{} {}..{}", "[>]".blue(), from_ref, to_ref);
    }

    // Get diff for chat sessions
    let mut diff_args = vec!["diff".to_string()];
    if to_ref.is_empty() {
        diff_args.push(from_ref.to_string());
    } else {
        diff_args.push(format!("{}..{}", from_ref, to_ref));
    }
    diff_args.push("--stat".to_string());
    diff_args.push("--".to_string());
    diff_args.push(".vscode/chat-sessions".to_string());

    let output = Command::new("git")
        .current_dir(project_dir)
        .args(&diff_args)
        .output()?;

    let chat_diff = String::from_utf8_lossy(&output.stdout);

    if chat_diff.is_empty() {
        println!("\n{} No changes to chat sessions", "[i]".blue());
    } else {
        println!("\n{} Chat session changes:", "[*]".cyan());
        for line in chat_diff.lines() {
            println!("   {}", line);
        }
    }

    // Show associated file changes if requested
    if with_files {
        let mut file_diff_args = vec!["diff".to_string()];
        if to_ref.is_empty() {
            file_diff_args.push(from_ref.to_string());
        } else {
            file_diff_args.push(format!("{}..{}", from_ref, to_ref));
        }
        file_diff_args.push("--stat".to_string());

        let output = Command::new("git")
            .current_dir(project_dir)
            .args(&file_diff_args)
            .output()?;

        let file_diff = String::from_utf8_lossy(&output.stdout);

        if !file_diff.is_empty() {
            println!("\n{} All file changes:", "[*]".cyan());
            for line in file_diff.lines().take(20) {
                println!("   {}", line);
            }
            if file_diff.lines().count() > 20 {
                println!("   ... and {} more", file_diff.lines().count() - 20);
            }
        }
    }

    Ok(())
}

/// Restore chat sessions from a specific commit
pub fn git_restore(project_path: &str, commit: &str, with_files: bool, backup: bool) -> Result<()> {
    let project_dir = Path::new(project_path);
    let chat_sessions_path = project_dir.join(".vscode").join("chat-sessions");

    println!("{} Restoring Chat Sessions", "[*]".blue().bold());
    println!("{}", "=".repeat(60));
    println!("{} From commit: {}", "[>]".blue(), commit);

    // Verify commit exists
    let output = Command::new("git")
        .current_dir(project_dir)
        .args(["rev-parse", "--verify", commit])
        .output()?;

    if !output.status.success() {
        anyhow::bail!("Commit not found: {}", commit);
    }

    // Create backup if requested
    if backup && chat_sessions_path.exists() {
        let backup_name = format!("chat-sessions-backup-{}", chrono::Utc::now().timestamp());
        let backup_path = project_dir.join(".vscode").join(&backup_name);

        if let Err(e) = std::fs::rename(&chat_sessions_path, &backup_path) {
            println!("{} Failed to create backup: {}", "[!]".yellow(), e);
        } else {
            println!("{} Created backup: {}", "[OK]".green(), backup_name);
        }
    }

    // Restore chat sessions
    let output = Command::new("git")
        .current_dir(project_dir)
        .args(["checkout", commit, "--", ".vscode/chat-sessions"])
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to restore chat sessions: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    println!("{} Restored chat sessions from {}", "[OK]".green(), commit);

    // Restore associated files if requested
    if with_files {
        println!(
            "\n{} This will restore ALL files from commit {}!",
            "[!]".yellow().bold(),
            commit
        );
        println!(
            "{} Are you sure? Use git checkout directly for selective restore.",
            "[i]".cyan()
        );
        // We don't actually restore all files - too dangerous
        // Instead, show what would be restored

        let output = Command::new("git")
            .current_dir(project_dir)
            .args(["diff", "--name-only", commit])
            .output()?;

        let files = String::from_utf8_lossy(&output.stdout);
        let file_count = files.lines().count();

        if file_count > 0 {
            println!("\n{} Files that differ from {}:", "[*]".cyan(), commit);
            for line in files.lines().take(10) {
                println!("   {}", line);
            }
            if file_count > 10 {
                println!("   ... and {} more", file_count - 10);
            }
            println!(
                "\n{} To restore all: git checkout {} -- .",
                "[i]".cyan(),
                commit
            );
        }
    }

    Ok(())
}
