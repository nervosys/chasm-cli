<p align="center">
  <h1 align="center">🗄️ Chasm CLI</h1>
  <p align="center">
    <strong>Universal Chat Session Manager</strong><br>
    Harvest, merge, and recover your AI chat history
  </p>
</p>

<p align="center">
  <a href="https://crates.io/crates/chasm-cli"><img src="https://img.shields.io/crates/v/chasm-cli.svg" alt="Crates.io"></a>
  <a href="https://docs.rs/chasm-cli"><img src="https://docs.rs/chasm-cli/badge.svg" alt="Documentation"></a>
  <a href="https://github.com/nervosys/chasm-cli/actions"><img src="https://github.com/nervosys/chasm-cli/workflows/CI/badge.svg" alt="CI Status"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="License"></a>
</p>

---

**Chasm** extracts and unifies chat sessions from AI coding assistants like GitHub Copilot, Cursor, and more. Never lose your AI conversations again.

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

| Platform | Download |
|----------|----------|
| Windows x64 | [chasm-v1.0.0-x86_64-pc-windows-msvc.zip](https://github.com/nervosys/chasm-cli/releases/latest) |
| Windows ARM | [chasm-v1.0.0-aarch64-pc-windows-msvc.zip](https://github.com/nervosys/chasm-cli/releases/latest) |
| macOS x64 | [chasm-v1.0.0-x86_64-apple-darwin.tar.gz](https://github.com/nervosys/chasm-cli/releases/latest) |
| macOS ARM | [chasm-v1.0.0-aarch64-apple-darwin.tar.gz](https://github.com/nervosys/chasm-cli/releases/latest) |
| Linux x64 | [chasm-v1.0.0-x86_64-unknown-linux-gnu.tar.gz](https://github.com/nervosys/chasm-cli/releases/latest) |
| Linux musl | [chasm-v1.0.0-x86_64-unknown-linux-musl.tar.gz](https://github.com/nervosys/chasm-cli/releases/latest) |

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

| Command | Description |
|---------|-------------|
| `chasm fetch path <project-path>` | **Recover sessions** - Fetches and registers sessions for a project |
| `chasm fetch workspace <pattern>` | Fetch sessions from workspaces matching a pattern |
| `chasm fetch session <id>` | Fetch a specific session by ID |
| `chasm register <path>` | Register orphaned sessions in VS Code''s database index |

### Listing & Discovery

| Command | Description |
|---------|-------------|
| `chasm list workspaces` | List all discovered workspaces |
| `chasm list sessions` | List all sessions |
| `chasm list sessions --project-path <path>` | List sessions for a specific project |
| `chasm detect all <path>` | Auto-detect workspace, providers, and sessions |
| `chasm detect workspace <path>` | Detect workspace info for a path |
| `chasm detect providers` | List available LLM providers |

### Viewing & Searching

| Command | Description |
|---------|-------------|
| `chasm show session <id>` | Display full session content |
| `chasm find session <pattern>` | Search sessions by text pattern |
| `chasm find workspace <pattern>` | Search workspaces by name |

### Export & Import

| Command | Description |
|---------|-------------|
| `chasm export path <dest> <project-path>` | Export sessions from a project |
| `chasm export workspace <dest> <hash>` | Export sessions from a workspace |
| `chasm import path <source> <project-path>` | Import sessions into a project workspace |

### Merging Sessions

| Command | Description |
|---------|-------------|
| `chasm merge path <project-path>` | Merge all sessions for a project into one |
| `chasm merge workspace <pattern>` | Merge sessions from matching workspaces |
| `chasm merge sessions <id1> <id2> ...` | Merge specific sessions by ID |
| `chasm merge all` | Merge all sessions across all providers |

### Harvesting (Bulk Collection)

| Command | Description |
|---------|-------------|
| `chasm harvest scan` | Scan for all available providers and sessions |
| `chasm harvest run` | Harvest sessions from all providers into database |
| `chasm harvest run --providers copilot` | Harvest only from specific providers |
| `chasm harvest status` | Show harvest database status |
| `chasm harvest search <query>` | Full-text search across all harvested sessions |

### Git Integration

| Command | Description |
|---------|-------------|
| `chasm git init` | Initialize git versioning for chat sessions |
| `chasm git add` | Stage and commit chat sessions |
| `chasm git status` | Show git status of chat sessions |
| `chasm git log` | Show history of chat session commits |
| `chasm git snapshot` | Create a tagged snapshot |

### Provider Management

| Command | Description |
|---------|-------------|
| `chasm provider list` | List discovered LLM providers |

### Server & API

| Command | Description |
|---------|-------------|
| `chasm api serve` | Start the REST API server |
| `chasm api serve --port 8787` | Start on specific port |

## 🔌 API Server

Start the REST API server for integration with web/mobile apps:

```bash
chasm api serve --host 0.0.0.0 --port 8787
```

### Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/health` | Health check |
| GET | `/api/workspaces` | List workspaces |
| GET | `/api/workspaces/:id` | Get workspace details |
| GET | `/api/sessions` | List sessions |
| GET | `/api/sessions/:id` | Get session with messages |
| GET | `/api/sessions/search?q=` | Search sessions |
| GET | `/api/stats` | Database statistics |
| GET | `/api/providers` | List supported providers |

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

| Platform | Location |
|----------|----------|
| Windows | `%LOCALAPPDATA%\csm\csm.db` |
| macOS | `~/Library/Application Support/csm/csm.db` |
| Linux | `~/.local/share/csm/csm.db` |

## 🛠️ Development

### Prerequisites

- Rust 1.75+
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

## �� Security

For security issues, please see our [Security Policy](SECURITY.md).

## 📞 Support

- 📖 [Documentation](https://docs.rs/chasm-cli)
- 💬 [GitHub Discussions](https://github.com/nervosys/chasm-cli/discussions)
- 🐛 [Issue Tracker](https://github.com/nervosys/chasm-cli/issues)

---

<p align="center">
  Made with ❤️ by <a href="https://nervosys.com">Nervosys</a>
</p>
