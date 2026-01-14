# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.2.0] - 2026-01-13

### Changed

- **BREAKING**: JWT secret (`CSM_JWT_SECRET`) is now required for API authentication
- **Security**: Upgraded password hashing from SHA-256 to Argon2id (OWASP recommended)
- Users with existing password hashes will need to reset their passwords

### Security

- Fixed weak password hashing (NIST FIPS compliance)
- Removed insecure development JWT secret fallback
- Added `verify_password` function using Argon2id
- PHC-format hashes now include embedded salts

### Dependencies

- Added `argon2` 0.5 for password hashing
- Removed `sha2` 0.10 (no longer needed)

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
