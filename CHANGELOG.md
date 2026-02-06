# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.3.2] - 2026-02-04

### Added

- **Real-time Session Recording API** - Prevent data loss from editor crashes
- **Multi-Provider Recording Support** - VS Code extension records from 10+ providers
  - `POST /api/recording/events` - Send recording events (SessionStart, MessageAdd, MessageAppend, Heartbeat)
  - `POST /api/recording/snapshot` - Store full session snapshot for recovery
  - `GET /api/recording/sessions` - List active recording sessions
  - `GET /api/recording/sessions/:id` - Get recorded session by ID
  - `GET /api/recording/recovery` - Recover sessions after crash
  - `GET /api/recording/status` - Recording service status
  - Event buffering with concurrent session tracking via `RecordingState`

## [1.1.0] - 2026-02-04

### Added

- **Multi-Provider Support for Forensic Tools** - Extend session forensics across all providers
  - Supported providers: VS Code (Copilot), Cursor, ClaudeCode, OpenCode, OpenClaw, Antigravity
  - `chasm list sessions --provider <name>` - Filter sessions by provider
  - `chasm list sessions --all-providers` - List sessions from all providers
  - `chasm list agents --provider <name>` - Filter agent sessions by provider
  - `chasm list agents -p all` - List agent sessions from all providers  
  - `chasm show timeline --provider <name>` - Show timeline for specific provider
  - `chasm show timeline --all-providers` - Aggregate timeline across all providers
  - `chasm find session --provider <name>` - Search within specific provider
  - `chasm find session --all-providers` - Search across all providers
  - Provider column added to output tables when multiple providers are shown
  - Provider aliases: `vscode`/`copilot`, `cursor`, `claudecode`/`claude`, `opencode`, `openclaw`/`claw`, `antigravity`/`ag`

- **JSONL Format Support** - Handle VS Code 1.109.0+ event-sourced session format
  - Automatic detection and parsing of `.jsonl` session files
  - Reconstruction of session state from event stream
  - Backward compatible with legacy JSON format

- **Agent Mode Session Tools**
  - `chasm list agents [--size]` - List Copilot Edits / chatEditingSessions
  - `chasm show agent <id>` - Show agent session details
  
- **Timeline Visualization**
  - `chasm show timeline [--agents]` - Visualize session activity with gap detection
  - Shows recent activity bars and identifies periods of inactivity
  - Helps identify missing or lost sessions

- **Session Search Enhancements**
  - `chasm find session --date YYYY-MM-DD` - Filter by internal message timestamp
  - `chasm find session --all` - Search across all workspaces
  - `chasm list sessions --size` - Show file size column

## [1.0.1] - 2026-01-17

### Added

- **Orphaned Session Detection** - Find and recover sessions from orphaned workspace hashes
  - `chasm detect orphaned [PATH]` - Scan for all workspace hashes matching a project path
  - Shows active vs orphaned workspaces with session counts and details
  - `--recover` flag automatically copies orphaned sessions to the active workspace
  - Helps recover valuable chat history when VS Code creates new workspace hashes

## [0.2.1] - 2025-01-10

### Added

- **MCP Server - CSM Database Integration** - Access csm-web database sessions
  - 5 new `csm_db_*` tools for csm-web database access:
    - `csm_db_list_workspaces` - List workspaces from CSM database
    - `csm_db_list_sessions` - List sessions with provider/workspace filters
    - `csm_db_get_session` - Get session with all messages
    - `csm_db_search` - Search sessions by title
    - `csm_db_stats` - Database statistics by provider
  - 3 new `csm://db/*` resources:
    - `csm://db/workspaces` - Workspaces resource
    - `csm://db/sessions` - Sessions resource  
    - `csm://db/stats` - Statistics resource
    - `csm://db/session/{id}` - Individual session resource

### Changed

- MCP server now supports both VS Code workspace storage AND csm-web database

## [0.2.0] - 2025-12-09

### Added

- **VS Code ALL SESSIONS Support** - Access workspace-independent sessions
  - `csm list workspaces` now shows "Empty window sessions (ALL SESSIONS)" count
  - `csm list sessions` includes ALL SESSIONS with "(ALL SESSIONS)" as project path
  - Sessions from VS Code's empty window are now discoverable and manageable

- **Harvest System** - Unified database for collecting sessions from all providers
  - `csm harvest init` - Initialize the harvest database
  - `csm harvest scan` - Scan for available providers and sessions
  - `csm harvest run` - Collect sessions from all providers
  - `csm harvest status` - Show database statistics
  - `csm harvest list` - List harvested sessions
  - `csm harvest export` - Export sessions from the database

- **Share Link Import** - Import shared chat sessions from web URLs
  - `csm harvest share <url>` - Register a share link for import
  - `csm harvest shares` - List pending or imported share links
  - Supports ChatGPT, Claude, Gemini, Perplexity, and Poe share URLs

- **Session Checkpoints** - Version snapshots for session tracking
  - `csm harvest checkpoint <session>` - Create a named checkpoint
  - `csm harvest checkpoints <session>` - List session checkpoints
  - `csm harvest restore <session> <checkpoint>` - Restore to checkpoint

- **Full-Text Search** - Search across all harvested messages
  - `csm harvest search <query>` - Search with FTS5 or LIKE fallback
  - `--provider` filter for provider-specific search
  - `--limit` to control result count

- **Universal Database Schema** (SQLite)
  - `sessions` - Session metadata with provider tracking
  - `messages` - Individual messages with full-text search
  - `checkpoints` - Version snapshots with content hashing
  - `share_links` - Pending and imported share URLs
  - `messages_fts` - FTS5 virtual table for fast search

- **Browser Integration** (foundation)
  - Browser profile discovery for Chrome, Edge, Firefox, Brave
  - Cookie extraction support (Windows DPAPI)

- **Auto-Detection Improvements**
  - `csm detect` - Enhanced workspace and provider detection
  - `csm detect all` - Comprehensive detection report
  - `csm detect providers --with-sessions` - Filter by active providers

### Changed

- Improved CLI output with ASCII-only characters for cross-platform compatibility
- Enhanced error messages with actionable suggestions
- Better handling of missing or inaccessible sessions

### Fixed

- Search query column name mismatch in harvest database
- Schema consistency between database.rs and harvest.rs
- Foreign key constraint issues in checkpoint tests

## [0.1.0] - 2025-11-15

### Added

- Initial release
- Workspace discovery and management
- Session import/export between workspaces
- History merging with chronological ordering
- Git integration for chat session versioning
- Migration tools for cross-machine transfers
- Interactive TUI browser
- Multi-provider support:
  - VS Code GitHub Copilot
  - Cursor IDE
  - Ollama
  - vLLM
  - Azure AI Foundry
  - LM Studio
  - LocalAI
  - Text Gen WebUI
  - Jan.ai
  - GPT4All
  - Llamafile
- Cross-platform support (Windows, macOS, Linux)
