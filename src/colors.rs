// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Terminal color scheme - Tokyo Night inspired
//!
//! Provides consistent styling across all CLI output.
//! Colors match the demo screenshot in assets/demo.svg.
//!
//! ## Tokyo Night Palette
//! - Blue (#7aa2f7) - prompts, indicators
//! - Green (#9ece6a) - success, OK, Yes
//! - Purple (#bb9af7) - headers, labels
//! - Cyan (#7dcfff) - hashes, paths, info
//! - Orange (#ff9e64) - counts, numbers
//! - Text (#c0caf5) - default text
//! - Dim (#565f89) - separators, borders

use colored::{ColoredString, Colorize};

/// Status indicators with consistent colors
pub struct Status;

impl Status {
    /// Success indicator: `[OK]` in green
    pub fn ok() -> ColoredString {
        "[OK]".green()
    }

    /// Info indicator: `[i]` in cyan
    pub fn info() -> ColoredString {
        "[i]".cyan()
    }

    /// Warning indicator: `[!]` in yellow/orange
    pub fn warn() -> ColoredString {
        "[!]".yellow()
    }

    /// Error indicator: `[X]` in red
    pub fn error() -> ColoredString {
        "[X]".red()
    }

    /// Progress/fetch indicator: `[<]` in blue
    pub fn fetch() -> ColoredString {
        "[<]".blue()
    }

    /// Action indicator: `[>]` in yellow
    pub fn action() -> ColoredString {
        "[>]".yellow()
    }

    /// Index/register indicator: `[#]` in blue
    pub fn index() -> ColoredString {
        "[#]".blue()
    }

    /// Detail indicator: `[*]` in blue
    pub fn detail() -> ColoredString {
        "[*]".blue()
    }

    /// Summary indicator: `[=]` in blue
    pub fn summary() -> ColoredString {
        "[=]".blue()
    }

    /// Detect indicator: `[D]` in blue bold
    pub fn detect() -> ColoredString {
        "[D]".blue().bold()
    }

    /// Add/found indicator: `[+]` in green bold
    pub fn add() -> ColoredString {
        "[+]".green().bold()
    }

    /// Package/restore indicator: `[P]` in blue
    pub fn package() -> ColoredString {
        "[P]".blue()
    }
}

/// Text styling helpers
pub trait StyledText {
    /// Style as a header/label (purple/magenta bold)
    fn header(&self) -> ColoredString;
    /// Style as a path/identifier (cyan)
    fn path(&self) -> ColoredString;
    /// Style as a hash/ID (cyan)
    fn hash(&self) -> ColoredString;
    /// Style as a count/number (yellow/orange)
    fn count(&self) -> ColoredString;
    /// Style as success value (green)
    fn success(&self) -> ColoredString;
    /// Style as a separator (dim gray)
    fn separator(&self) -> ColoredString;
    /// Style as an error (red)
    fn err(&self) -> ColoredString;
    /// Style as a warning (yellow)
    fn warning(&self) -> ColoredString;
    /// Style as branding (cyan bold)
    fn brand(&self) -> ColoredString;
}

impl StyledText for str {
    fn header(&self) -> ColoredString {
        self.magenta().bold()
    }

    fn path(&self) -> ColoredString {
        self.cyan()
    }

    fn hash(&self) -> ColoredString {
        self.cyan()
    }

    fn count(&self) -> ColoredString {
        self.yellow()
    }

    fn success(&self) -> ColoredString {
        self.green()
    }

    fn separator(&self) -> ColoredString {
        self.dimmed()
    }

    fn err(&self) -> ColoredString {
        self.red()
    }

    fn warning(&self) -> ColoredString {
        self.yellow()
    }

    fn brand(&self) -> ColoredString {
        self.cyan().bold()
    }
}

impl StyledText for String {
    fn header(&self) -> ColoredString {
        self.as_str().magenta().bold()
    }

    fn path(&self) -> ColoredString {
        self.as_str().cyan()
    }

    fn hash(&self) -> ColoredString {
        self.as_str().cyan()
    }

    fn count(&self) -> ColoredString {
        self.as_str().yellow()
    }

    fn success(&self) -> ColoredString {
        self.as_str().green()
    }

    fn separator(&self) -> ColoredString {
        self.as_str().dimmed()
    }

    fn err(&self) -> ColoredString {
        self.as_str().red()
    }

    fn warning(&self) -> ColoredString {
        self.as_str().yellow()
    }

    fn brand(&self) -> ColoredString {
        self.as_str().cyan().bold()
    }
}

/// Create a horizontal separator line
pub fn separator(width: usize) -> ColoredString {
    "═".repeat(width).dimmed()
}

/// Create a simple separator line
pub fn line(width: usize) -> ColoredString {
    "=".repeat(width).dimmed()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_indicators() {
        // Just verify they compile and return ColoredString
        let _ = Status::ok();
        let _ = Status::info();
        let _ = Status::warn();
        let _ = Status::error();
        let _ = Status::fetch();
    }

    #[test]
    fn test_styled_text() {
        let text = "test";
        let _ = text.header();
        let _ = text.path();
        let _ = text.hash();
        let _ = text.count();
        let _ = text.success();
    }
}
