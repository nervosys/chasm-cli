<p align="center">
  <img src="https://raw.githubusercontent.com/nervosys/chasm-cli/master/assets/banner.png" alt="Chasm" width="100%">
</p>

<p align="center">
  <strong>Chat Session Manager (Chasm): Bridging the divide between AI providers</strong><br>
  <em>Harvest, harmonize, and recover your AI chat and agent task histories</em>
</p>

<p align="center">
  <a href="https://crates.io/crates/chasm-cli"><img src="https://img.shields.io/crates/v/chasm-cli.svg?style=flat-square&logo=rust&logoColor=white&color=orange" alt="Crates.io"></a>
  <a href="https://docs.rs/chasm-cli"><img src="https://img.shields.io/docsrs/chasm-cli?style=flat-square&logo=docs.rs&logoColor=white" alt="Documentation"></a>
  <a href="https://github.com/nervosys/chasm-cli/actions"><img src="https://img.shields.io/github/actions/workflow/status/nervosys/chasm-cli/ci.yml?style=flat-square&logo=github&logoColor=white&label=CI" alt="CI Status"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg?style=flat-square" alt="License"></a>
  <a href="https://github.com/nervosys/chasm-cli/releases"><img src="https://img.shields.io/github/v/release/nervosys/chasm-cli?style=flat-square&logo=github&logoColor=white&label=release" alt="Release"></a>
</p>

<p align="center">
  <a href="#-quick-start">Quick Start</a> •
  <a href="#-features">Features</a> •
  <a href="#-installation">Installation</a> •
  <a href="#-documentation">Docs</a> •
  <a href="#-contributing">Contributing</a>
</p>

<br>

<p align="center">
  <img src="https://raw.githubusercontent.com/nervosys/chasm-cli/master/assets/demo.svg" alt="Chasm Demo" width="850">
</p>

<br>

---

**Chasm** bridges the divide between AI providers by extracting and unifying chat sessions from AI coding assistants like GitHub Copilot, Cursor, and more. Never lose your AI conversations again.

## ✨ Features

- 🔍 **Harvest** - Extract chat sessions from VS Code, Cursor, Windsurf, and other editors
- 🔄 **Recover** - Restore lost or orphaned chat sessions to VS Code
- 🔀 **Merge** - Combine sessions across workspaces and time periods
- 📊 **Analyze** - Get statistics on your AI assistant usage
- 🔌 **API Server** - REST API for building custom integrations
- 🤖 **MCP Tools** - Model Context Protocol support for AI agent integration
- 🗃️ **Universal Database** - SQLite-based storage that normalizes all providers

## 📦 Installation

### From crates.io

```bash
cargo install chasm-cli
```

### From source

```bash
git clone https://github.com/nervosys/chasm-cli.git
cd chasm-cli
cargo install --path .
```

### Pre-built binaries

Download from [GitHub Releases](https://github.com/nervosys/chasm-cli/releases):

| Platform    | Download                                                                                               |
| ----------- | ------------------------------------------------------------------------------------------------------ |
| Windows x64 | [chasm-v1.0.0-x86_64-pc-windows-msvc.zip](https://github.com/nervosys/chasm-cli/releases/latest)       |
| Windows ARM | [chasm-v1.0.0-aarch64-pc-windows-msvc.zip](https://github.com/nervosys/chasm-cli/releases/latest)      |
| macOS x64   | [chasm-v1.0.0-x86_64-apple-darwin.tar.gz](https://github.com/nervosys/chasm-cli/releases/latest)       |
| macOS ARM   | [chasm-v1.0.0-aarch64-apple-darwin.tar.gz](https://github.com/nervosys/chasm-cli/releases/latest)      |
| Linux x64   | [chasm-v1.0.0-x86_64-unknown-linux-gnu.tar.gz](https://github.com/nervosys/chasm-cli/releases/latest)  |
| Linux musl  | [chasm-v1.0.0-x86_64-unknown-linux-musl.tar.gz](https://github.com/nervosys/chasm-cli/releases/latest) |

## 🚀 Quick Start

### Recover Lost Chat Sessions

The most common use case - recover chat sessions that disappeared from VS Code:

```bash
# Recover sessions for a specific project
chasm fetch path /path/to/your/project

# Example output:
# [<] Fetching Chat History for: my-project
# ======================================================================
# Found 3 historical workspace(s)
#
#    [OK] Fetched: Implementing authentication system... (abc12345-...)
#    [OK] Fetched: Debugging API endpoints... (def67890-...)
#
# ======================================================================
# Fetched: 2 sessions
#
# [i] Reload VS Code (Ctrl+R) and check Chat history dropdown
```

After running, **reload VS Code** (`Ctrl+R` or `Cmd+R`) and your sessions will appear in the Chat history dropdown.

### Auto-Detect Workspace Info

```bash
# See what chasm knows about a workspace
chasm detect all /path/to/your/project --verbose

# Output shows:
# - Workspace ID and status
# - Available sessions
# - Detected providers
# - Recommendations
```

### List All Workspaces

```bash
chasm list workspaces
```

```
┌──────────────────┬──────────────────────────────────────────┬──────────┬───────────┐
│ Hash             │ Project Path                             │ Sessions │ Has Chats │
├──────────────────┼──────────────────────────────────────────┼──────────┼───────────┤
│ 91d41f3d61f1...  │ c:\dev\my-project                        │ 3        │ Yes       │
│ a2b3c4d5e6f7...  │ c:\dev\another-project                   │ 1        │ Yes       │
└──────────────────┴──────────────────────────────────────────┴──────────┴───────────┘
```

### List Sessions for a Project

```bash
chasm list sessions --project-path /path/to/your/project
```

### Search for Sessions

```bash
# Find sessions by project name
chasm find session "my-project"

# Find sessions containing specific text
chasm find session "authentication"
```

### View Session Details

```bash
chasm show session <session-id>
```

### Export Sessions

```bash
# Export to a backup directory
chasm export path /backup/dir /path/to/your/project
```

## 📖 Complete CLI Reference

### Session Recovery & Fetching

| Command                            | Description                                                         |
| ---------------------------------- | ------------------------------------------------------------------- |
| `chasm fetch path <project-path>`  | **Recover sessions** - Fetches and registers sessions for a project |
| `chasm fetch workspace <pattern>`  | Fetch sessions from workspaces matching a pattern                   |
| `chasm fetch session <id>`         | Fetch a specific session by ID                                      |
| `chasm register all --path <path>` | Register all on-disk sessions into VS Code's database index         |

### Listing & Discovery

| Command                                     | Description                                        |
| ------------------------------------------- | -------------------------------------------------- |
| `chasm list workspaces`                     | List all discovered workspaces                     |
| `chasm list sessions`                       | List all sessions                                  |
| `chasm list sessions --project-path <path>` | List sessions for a specific project               |
| `chasm detect all <path>`                   | Auto-detect workspace, providers, and sessions     |
| `chasm detect workspace <path>`             | Detect workspace info for a path                   |
| `chasm detect providers`                    | List available LLM providers                       |
| `chasm detect orphaned <path>`              | Find orphaned workspaces with recoverable sessions |
| `chasm detect orphaned --recover <path>`    | Recover orphaned sessions to the active workspace  |

### Viewing & Searching

| Command                          | Description                     |
| -------------------------------- | ------------------------------- |
| `chasm show session <id>`        | Display full session content    |
| `chasm find session <pattern>`   | Search sessions by text pattern |
| `chasm find workspace <pattern>` | Search workspaces by name       |

### Export & Import

| Command                                     | Description                              |
| ------------------------------------------- | ---------------------------------------- |
| `chasm export path <dest> <project-path>`   | Export sessions from a project           |
| `chasm export workspace <dest> <hash>`      | Export sessions from a workspace         |
| `chasm import path <source> <project-path>` | Import sessions into a project workspace |

### Merging Sessions

| Command                                | Description                               |
| -------------------------------------- | ----------------------------------------- |
| `chasm merge path <project-path>`      | Merge all sessions for a project into one |
| `chasm merge workspace <pattern>`      | Merge sessions from matching workspaces   |
| `chasm merge sessions <id1> <id2> ...` | Merge specific sessions by ID             |
| `chasm merge all`                      | Merge all sessions across all providers   |

### Sync & Recovery

| Command                                   | Description                                               |
| ----------------------------------------- | --------------------------------------------------------- |
| `chasm sync --pull`                       | Pull sessions from provider workspaces into database      |
| `chasm sync --push`                       | Push sessions from database back to provider workspaces   |
| `chasm sync --pull --push`                | Bidirectional sync                                        |
| `chasm sync --pull --workspace <pattern>` | Sync only matching workspaces                             |
| `chasm recover scan`                      | Scan for recoverable sessions from various sources        |
| `chasm recover extract <path>`            | Extract sessions from a VS Code workspace by project path |
| `chasm recover orphans`                   | List sessions that may be orphaned in workspaceStorage    |
| `chasm recover repair`                    | Repair corrupted session files in place                   |
| `chasm recover convert`                   | Convert session files between JSON and JSONL formats      |
| `chasm recover status`                    | Show recovery status and recommendations                  |
| `chasm export batch <dest> <paths...>`    | Batch export sessions from multiple projects              |

### Harvesting (Bulk Collection)

| Command                                 | Description                                       |
| --------------------------------------- | ------------------------------------------------- |
| `chasm harvest scan`                    | Scan for all available providers and sessions     |
| `chasm harvest run`                     | Harvest sessions from all providers into database |
| `chasm harvest run --providers copilot` | Harvest only from specific providers              |
| `chasm harvest status`                  | Show harvest database status                      |
| `chasm harvest search <query>`          | Full-text search across all harvested sessions    |
| `chasm harvest sync --push`             | Alias for `chasm sync --push`                     |
| `chasm harvest sync --pull`             | Alias for `chasm sync --pull`                     |

### Git Integration

| Command              | Description                                 |
| -------------------- | ------------------------------------------- |
| `chasm git init`     | Initialize git versioning for chat sessions |
| `chasm git add`      | Stage and commit chat sessions              |
| `chasm git status`   | Show git status of chat sessions            |
| `chasm git log`      | Show history of chat session commits        |
| `chasm git snapshot` | Create a tagged snapshot                    |

### Provider Management

| Command               | Description                   |
| --------------------- | ----------------------------- |
| `chasm provider list` | List discovered LLM providers |

### Server & API

| Command                       | Description               |
| ----------------------------- | ------------------------- |
| `chasm api serve`             | Start the REST API server |
| `chasm api serve --port 8787` | Start on specific port    |

### Telemetry

| Command               | Description                            |
| --------------------- | -------------------------------------- |
| `chasm telemetry`     | Show current telemetry status          |
| `chasm telemetry on`  | Enable anonymous usage data collection |
| `chasm telemetry off` | Disable telemetry (opt-in by default)  |


## 🤖 Agency - Agentic Coding CLI

Chasm includes a full **agentic coding toolkit** similar to Claude Code CLI, but provider-agnostic. Run coding tasks with any LLM backend.

### Available Tools

```bash
chasm agency tools
```

| Tool           | Description                    |
| -------------- | ------------------------------ |
| `file_read`    | Read file contents             |
| `file_write`   | Write or modify files          |
| `terminal`     | Execute shell commands         |
| `code_search`  | Search codebase for symbols    |
| `web_search`   | Search the web for information |
| `http_request` | Make HTTP requests             |
| `calculator`   | Perform calculations           |

### Agent Roles

```bash
chasm agency list
```

- **coordinator** - Orchestrates multi-agent workflows
- **coder** - Writes and refactors code
- **reviewer** - Reviews code for issues
- **tester** - Generates and runs tests
- **researcher** - Gathers information
- **executor** - Runs commands and tasks

### Orchestration Modes

```bash
chasm agency modes
```

| Mode           | Description                                 |
| -------------- | ------------------------------------------- |
| `single`       | Traditional single-agent (like Claude Code) |
| `sequential`   | Agents execute one after another            |
| `parallel`     | Multiple agents work simultaneously         |
| `swarm`        | Coordinated multi-agent collaboration       |
| `hierarchical` | Lead agent delegates to specialists         |
| `debate`       | Agents debate to find best solution         |

### Usage Examples

```bash
# Simple coding task (single agent)
chasm agency run "Add error handling to main.rs"

# Specify a model
chasm agency run -m gpt-4o "Refactor this function to use async/await"

# Use local Ollama model
chasm agency run -m ollama/codellama "Write unit tests for lib.rs"

# Multi-agent swarm for complex tasks
chasm agency run --orchestration swarm "Build a REST API with authentication"

# Parallel agents for speed
chasm agency run --orchestration parallel "Analyze and fix all TODO comments"
```

## 🔄 Unified Chat Interface - No Vendor Lock-in

Chasm provides a **unified interface to all chat systems**, preventing vendor lock-in. Continue conversations seamlessly across providers.

### The Problem

Your AI chat history is scattered across:
- VS Code Copilot (SQLite + JSON in workspaceStorage)
- Cursor (proprietary format)
- ChatGPT (web-only, export required)
- Claude (web-only)
- Local LLMs (various formats)

Each uses different formats, storage locations, and APIs. If you switch providers, you lose context.

### The Solution

Chasm normalizes all sessions into a **universal format** and lets you:

1. **Import from any provider** into a unified database
2. **Export to any format** (JSON, Markdown, CSV)
3. **Continue sessions** with a different provider
4. **Search across all history** regardless of source

### Continue a Session with Any Provider

```bash
# List all your sessions from all providers
chasm list sessions

# Export a Copilot session
chasm export sessions abc123 --format json --output session.json

# The exported session contains the full conversation:
# - All messages (user + assistant)
# - Tool invocations and results
# - Timestamps and metadata
# - File references and code blocks

# Continue the conversation with a different provider:
chasm agency run --context session.json "Continue implementing the feature"

# Or import into the harvest database for unified access
chasm harvest import session.json
```

### Cross-Provider Workflow Example

```bash
# 1. Start a project with GitHub Copilot in VS Code
#    (sessions automatically tracked)

# 2. Later, recover and view your sessions
chasm fetch path /path/to/project
chasm list sessions --project-path /path/to/project

# 3. Export the session for portability
chasm export path ./backup /path/to/project

# 4. Continue with Claude, GPT-4, or local Ollama
chasm agency run -m claude-3 --context ./backup/session.json \
  "Review the code we wrote and suggest improvements"

# 5. Or merge multiple sessions into one unified history
chasm merge path /path/to/project

# 6. Search across ALL your AI conversations
chasm harvest search "authentication implementation"
```

### Universal Session Format

Chasm's normalized format includes:

```json
{
  "id": "uuid",
  "title": "Session title",
  "provider": "copilot|cursor|chatgpt|claude|ollama|...",
  "workspace": "/path/to/project",
  "created_at": "2026-01-08T12:00:00Z",
  "messages": [
    {
      "role": "user|assistant|system",
      "content": "Message text",
      "timestamp": "2026-01-08T12:00:00Z",
      "tool_calls": [...],
      "references": [...]
    }
  ],
  "metadata": {
    "model": "gpt-4o",
    "total_tokens": 15000,
    "files_referenced": ["src/main.rs", "Cargo.toml"]
  }
}
```

### Benefits

| Feature              | Vendor Lock-in    | With Chasm                  |
| -------------------- | ----------------- | --------------------------- |
| Switch providers     | Lose all history  | Keep everything             |
| Search old chats     | Per-provider only | Search all at once          |
| Backup conversations | Manual exports    | Automatic harvesting        |
| Continue sessions    | Start fresh       | Full context preserved      |
| Compare providers    | Impossible        | Same task, different models |


## 🔌 API Server

Start the REST API server for integration with web/mobile apps:

```bash
chasm api serve --host 0.0.0.0 --port 8787
```

### Endpoints

| Method | Endpoint                  | Description               |
| ------ | ------------------------- | ------------------------- |
| GET    | `/api/health`             | Health check              |
| GET    | `/api/workspaces`         | List workspaces           |
| GET    | `/api/workspaces/:id`     | Get workspace details     |
| GET    | `/api/sessions`           | List sessions             |
| GET    | `/api/sessions/:id`       | Get session with messages |
| GET    | `/api/sessions/search?q=` | Search sessions           |
| GET    | `/api/stats`              | Database statistics       |
| GET    | `/api/providers`          | List supported providers  |

## 🗃️ Supported Providers

### Editor-based
- ✅ GitHub Copilot (VS Code)
- ✅ Cursor
- ✅ Windsurf
- ✅ Continue.dev

### Local LLMs
- ✅ Ollama
- ✅ LM Studio
- ✅ GPT4All
- ✅ LocalAI
- ✅ Jan.ai
- ✅ llama.cpp / llamafile
- ✅ vLLM
- ✅ Text Generation WebUI

### Cloud APIs
- ✅ OpenAI / ChatGPT
- ✅ Anthropic / Claude
- ✅ Google / Gemini
- ✅ Azure AI Foundry
- ✅ Perplexity
- ✅ DeepSeek

## 📁 Database Locations

| Platform | Location                                   |
| -------- | ------------------------------------------ |
| Windows  | `%LOCALAPPDATA%\csm\csm.db`                |
| macOS    | `~/Library/Application Support/csm/csm.db` |
| Linux    | `~/.local/share/csm/csm.db`                |

## 🛠️ Development

### Prerequisites

- Rust 1.85+
- Git

### Building

```bash
git clone https://github.com/nervosys/chasm-cli.git
cd chasm-cli
cargo build --release
```

### Running tests

```bash
cargo test
```

## 📜 License

Licensed under the Apache License, Version 2.0 ([LICENSE](LICENSE) or http://www.apache.org/licenses/LICENSE-2.0).

## 🤝 Contributing

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) and [Code of Conduct](CODE_OF_CONDUCT.md).

## 🔒 Security

For security issues, please see our [Security Policy](SECURITY.md).
### Security Audit Summary (v1.2.9)

Chasm underwent a comprehensive security audit in January 2026 against industry frameworks:

| Framework        | Status      | Notes                           |
| ---------------- | ----------- | ------------------------------- |
| **CVE/RustSec**  | ✅ Pass      | No direct vulnerabilities       |
| **MITRE ATT&CK** | ✅ Mitigated | Command execution requires auth |
| **NIST FIPS**    | ✅ Compliant | Argon2id password hashing       |
| **CMMC 2.0**     | ✅ Compliant | Authentication hardened         |

**Key Security Features:**
- 🔐 **Argon2id** password hashing (OWASP recommended)
- 🔑 **JWT authentication** with required secrets (no dev fallbacks)
- 🛡️ **Parameterized SQL** queries (no injection vectors)
- 🔒 **DPAPI/Keychain** integration for credential access

**Dependencies:** 2 transitive advisories from `ratatui` TUI framework (`paste` unmaintained, `lru` unsound iterator) - compile-time/TUI only, no runtime security risk.

## 📞 Support

- 📖 [Documentation](https://docs.rs/chasm-cli)
- 💬 [GitHub Discussions](https://github.com/nervosys/chasm-cli/discussions)
- 🐛 [Issue Tracker](https://github.com/nervosys/chasm-cli/issues)
- 📧 [Email Support](mailto:support@nervosys.com)

---

<p align="center">
  <sub>Built with Rust 🦀 by <a href="https://nervosys.ai">NERVOSYS</a></sub>
</p>

<p align="center">
  <a href="https://github.com/nervosys/chasm-cli/stargazers">⭐ Star us on GitHub</a>
</p>
