# Contributing to Chasm

Thank you for your interest in contributing to Chasm! This document provides guidelines and information for contributors.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [How to Contribute](#how-to-contribute)
- [Development Setup](#development-setup)
- [Pull Request Process](#pull-request-process)
- [Coding Standards](#coding-standards)
- [Commit Messages](#commit-messages)
- [License](#license)

## Code of Conduct

This project adheres to a [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report unacceptable behavior to [security@nervosys.ai](mailto:security@nervosys.ai).

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/chasm-cli.git
   cd chasm-cli
   ```
3. **Add the upstream remote**:
   ```bash
   git remote add upstream https://github.com/nervosys/chasm-cli.git
   ```

## How to Contribute

### Reporting Bugs

Before creating a bug report, please check existing issues to avoid duplicates.

When filing a bug report, include:
- A clear, descriptive title
- Steps to reproduce the issue
- Expected vs. actual behavior
- Your environment (OS, Rust version, etc.)
- Relevant logs or error messages

### Suggesting Features

Feature requests are welcome! Please:
- Check if the feature has already been requested
- Describe the use case and why it would be valuable
- Consider if it aligns with the project's goals

### Contributing Code

1. Check for existing issues or create one to discuss your proposed change
2. Fork the repository and create a branch from `main`
3. Write your code following our [coding standards](#coding-standards)
4. Add or update tests as needed
5. Update documentation if required
6. Submit a pull request

## Development Setup

### Prerequisites

- Rust 1.85+ (install via [rustup](https://rustup.rs/))
- Git

### Building

```bash
# Build in debug mode
cargo build

# Build in release mode
cargo build --release

# Run the CLI
cargo run -- --help

# Run tests
cargo test

# Run with specific features
cargo build --features "feature_name"
```

### Project Structure

```
chasm/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── lib.rs           # Library exports
│   ├── cli.rs           # CLI argument definitions
│   ├── commands/        # CLI command implementations
│   ├── providers/       # Chat provider integrations
│   ├── database.rs      # SQLite database operations
│   ├── api/             # REST API server
│   └── mcp/             # MCP tool server
├── tests/               # Integration tests
├── examples/            # Example code
└── docs/                # Documentation
```

## Pull Request Process

1. **Create a descriptive PR title** that summarizes the change
2. **Fill out the PR template** completely
3. **Ensure CI passes** - all tests must pass
4. **Request review** from maintainers
5. **Address feedback** promptly and professionally
6. **Squash commits** if requested before merge

### PR Checklist

- [ ] Code compiles without warnings (`cargo build`)
- [ ] All tests pass (`cargo test`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] Lints pass (`cargo clippy`)
- [ ] Documentation updated if needed
- [ ] CHANGELOG.md updated for user-facing changes

## Coding Standards

### Rust Style

- Follow the [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/)
- Use `cargo fmt` before committing
- Address all `cargo clippy` warnings
- Write idiomatic Rust code

### Documentation

- Document all public APIs with doc comments
- Include examples in doc comments where helpful
- Keep comments up to date with code changes

### Testing

- Write unit tests for new functionality
- Write integration tests for CLI commands
- Aim for good test coverage on critical paths
- Use descriptive test names

```rust
#[test]
fn test_workspace_list_returns_sorted_results() {
    // Test implementation
}
```

### Error Handling

- Use `anyhow::Result` for application errors
- Use `thiserror` for library errors with specific types
- Provide helpful error messages
- Don't panic in library code

## Commit Messages

Follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Formatting, no code change
- `refactor`: Code change that neither fixes a bug nor adds a feature
- `perf`: Performance improvement
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

### Examples

```
feat(providers): add support for Claude chat export

fix(cli): handle empty workspace paths gracefully

docs(readme): update installation instructions

refactor(database): extract session queries into module
```

## Adding a New Provider

To add support for a new chat provider:

1. Create a new file in `src/providers/`
2. Implement the `ChatProvider` trait
3. Add the provider to `src/providers/mod.rs`
4. Add tests in `tests/provider_tests.rs`
5. Update documentation

See existing providers for reference implementations.

## License

By contributing to Chasm, you agree that your contributions will be licensed under the Apache License 2.0.

### Developer Certificate of Origin

By submitting a contribution, you certify that:

1. The contribution was created in whole or in part by you and you have the right to submit it under the open source license indicated in the file; or

2. The contribution is based upon previous work that, to the best of your knowledge, is covered under an appropriate open source license and you have the right under that license to submit that work with modifications; or

3. The contribution was provided directly to you by some other person who certified (1) or (2) and you have not modified it.

You can signify your acceptance of the DCO by adding a "Signed-off-by" line to your commit messages:

```
Signed-off-by: Your Name <your.email@example.com>
```

## Questions?

- Open a [GitHub Discussion](https://github.com/nervosys/chasm-cli/discussions) for general questions
- Check [existing issues](https://github.com/nervosys/chasm-cli/issues) for known problems
- Read the [documentation](https://docs.rs/chasm-cli) for API details

Thank you for contributing to Chasm! 🎉


