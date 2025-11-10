# Atlas Developer Guide

This guide provides information for developers who want to contribute to Atlas or extend its functionality.

## Table of Contents

- [Development Setup](#development-setup)
- [Building and Testing](#building-and-testing)
- [Code Organization](#code-organization)
- [Adding New Features](#adding-new-features)
- [Testing Guidelines](#testing-guidelines)
- [Code Style and Standards](#code-style-and-standards)
- [Contributing Guidelines](#contributing-guidelines)
- [Release Process](#release-process)

## Development Setup

### Prerequisites

1. **Rust 1.70 or later**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup update
   ```

2. **Development Tools**
   ```bash
   # Install clippy for linting
   rustup component add clippy

   # Install rustfmt for code formatting
   rustup component add rustfmt

   # Install cargo-watch for auto-rebuild (optional)
   cargo install cargo-watch

   # Install cargo-audit for security audits (optional)
   cargo install cargo-audit
   ```

3. **IDE Setup** (recommended)
   - **VS Code**: Install `rust-analyzer` extension
   - **IntelliJ IDEA**: Install Rust plugin
   - **Vim/Neovim**: Install `rust.vim` and `coc-rust-analyzer`

### Clone and Build

```bash
# Clone the repository
git clone https://github.com/erikhoward/atlas.git
cd atlas

# Build in debug mode
cargo build

# Build in release mode
cargo build --release

# Run tests
cargo test

# Run clippy
cargo clippy --all-targets -- -D warnings

# Format code
cargo fmt
```

### Development Environment

Create a development configuration file:

```bash
# Generate sample config
./target/debug/atlas init --with-examples --output atlas-dev.toml

# Edit for your development environment
vi atlas-dev.toml
```

Set up environment variables:

```bash
# Create .env file (add to .gitignore)
cat > .env << EOF
ATLAS_OPENEHR_USERNAME=dev-username
ATLAS_OPENEHR_PASSWORD=dev-password
ATLAS_COSMOSDB_KEY=dev-cosmos-key
EOF

# Load environment variables
source .env
```

### Running in Development Mode

```bash
# Run with debug logging
cargo run -- export -c atlas-dev.toml --log-level debug

# Run with dry-run mode
cargo run -- export -c atlas-dev.toml --dry-run

# Watch for changes and auto-rebuild
cargo watch -x 'run -- export -c atlas-dev.toml --dry-run'
```

## Building and Testing

### Build Commands

```bash
# Debug build (faster compilation, slower runtime)
cargo build

# Release build (slower compilation, optimized runtime)
cargo build --release

# Build specific binary
cargo build --bin atlas

# Build with all features
cargo build --all-features

# Check without building
cargo check
```

### Testing Commands

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run tests for specific module
cargo test --lib core::transform

# Run integration tests only
cargo test --test '*'

# Run with coverage (requires cargo-tarpaulin)
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

### Linting and Formatting

```bash
# Run clippy (linter)
cargo clippy --all-targets -- -D warnings

# Fix clippy warnings automatically (where possible)
cargo clippy --fix

# Format code
cargo fmt

# Check formatting without modifying files
cargo fmt -- --check
```

### Documentation

```bash
# Generate documentation
cargo doc --no-deps

# Generate and open documentation in browser
cargo doc --no-deps --open

# Check documentation links
cargo doc --no-deps 2>&1 | grep warning
```

## Key Implementation Details

### OpenEHR REST API Integration

Atlas implements the openEHR REST API specification for fetching compositions. Key endpoints:

**Fetch Composition** (requires both `ehr_id` and `composition_uid`):
```
GET /ehr/{ehr_id}/composition/{uid_based_id}?format=FLAT
```

**Important Notes**:
- The `ehr_id` is **required** in the URL path (not just the composition UID)
- The `format=FLAT` query parameter requests FLAT JSON format
- The composition UID includes version (e.g., `ac711c86-2d52-4260-837b-3fa040782287::local.ehrbase.org::1`)

**Example**:
```
GET /ehr/0be33a43-1f9b-4c1b-9a93-94c36f401570/composition/ac711c86-2d52-4260-837b-3fa040782287::local.ehrbase.org::1?format=FLAT
```

The `fetch_composition` method in the `OpenEhrVendor` trait accepts `&CompositionMetadata` which contains all required fields:
- `uid`: CompositionUid
- `ehr_id`: EhrId (required for URL construction)
- `template_id`: TemplateId (required for Composition builder)
- `time_committed`: DateTime<Utc> (required for Composition builder)

### Export Summary Tracking

The export summary tracks detailed statistics about the export process:

**BatchResult → ExportSummary Flow**:
1. `BatchProcessor::process_batch()` returns a `BatchResult` with counts:
   - `successful`: Compositions successfully inserted
   - `failed`: Compositions that failed transformation or insertion
   - `duplicates_skipped`: Compositions already in Cosmos DB
   - `errors`: List of error messages

2. `ExportCoordinator::process_ehr_for_template()` merges batch results into the summary:
   ```rust
   summary.total_compositions += batch_result.successful + batch_result.failed;
   summary.successful_exports += batch_result.successful;
   summary.failed_exports += batch_result.failed;
   summary.duplicates_skipped += batch_result.duplicates_skipped;
   ```

3. The final summary provides accurate counts for reporting

**Important**: Always capture and use the `BatchResult` returned from `process_batch()` - don't ignore it with `let _batch_result = ...`

### TLS Certificate Verification

Atlas implements security-focused TLS certificate verification with environment-aware enforcement.

#### Configuration

Both `tls_verify` and `tls_verify_certificates` are supported as aliases for backward compatibility:

```rust
// In src/config/schema.rs
pub struct OpenEhrConfig {
    pub tls_verify: bool,
    pub tls_verify_certificates: bool,
    // ...
}
```

#### Runtime Warning

When TLS verification is disabled, a security warning is logged at WARN level during client initialization:

```rust
// In src/adapters/openehr/vendor/ehrbase.rs
if !config.tls_verify || !config.tls_verify_certificates {
    tracing::warn!(
        "⚠️  SECURITY WARNING: TLS certificate verification is DISABLED for OpenEHR server at {}. \
        This configuration is INSECURE and should only be used in development/testing environments. \
        The application is vulnerable to man-in-the-middle attacks. \
        For production use, either enable TLS verification (tls_verify = true) or provide a custom CA certificate (tls_ca_cert).",
        config.base_url
    );
    client_builder = client_builder.danger_accept_invalid_certs(true);
}
```

#### Production Enforcement

Configuration validation enforces TLS verification in production environments:

```rust
// In src/config/schema.rs - OpenEhrConfig::validate()
if *environment == Environment::Production
    && (!self.tls_verify || !self.tls_verify_certificates)
{
    return Err(
        "TLS certificate verification cannot be disabled in production environments. \
        This is a critical security requirement to prevent man-in-the-middle attacks. \
        Either set 'tls_verify = true' or provide a custom CA certificate using 'tls_ca_cert'. \
        For development/testing environments, set 'environment = \"development\"' or 'environment = \"staging\"'.".to_string()
    );
}
```

#### Environment Configuration

The `environment` field in `AtlasConfig` controls security policies:

```rust
pub enum Environment {
    Development,  // TLS verification optional (with warning)
    Staging,      // TLS verification optional (with warning)
    Production,   // TLS verification required (enforced)
}
```

This three-tier approach ensures:
1. **Development flexibility**: Developers can work with self-signed certificates
2. **Security awareness**: Warnings alert users to insecure configurations
3. **Production safety**: Insecure configurations are blocked in production

## Code Organization

Atlas follows a layered architecture. Here's the directory structure:

```
atlas/
├── src/
│   ├── main.rs              # Binary entry point
│   ├── lib.rs               # Library root
│   │
│   ├── cli/                 # CLI layer
│   │   ├── mod.rs           # CLI module root
│   │   └── commands/        # Command implementations
│   │       ├── export.rs    # Export command
│   │       ├── validate.rs  # Validate command
│   │       ├── status.rs    # Status command
│   │       └── init.rs      # Init command
│   │
│   ├── core/                # Business logic layer
│   │   ├── export/          # Export orchestration
│   │   │   ├── coordinator.rs
│   │   │   ├── batch.rs
│   │   │   └── summary.rs
│   │   ├── transform/       # Data transformation
│   │   │   ├── preserve.rs
│   │   │   ├── flatten.rs
│   │   │   └── mod.rs
│   │   ├── state/           # State management
│   │   │   ├── manager.rs
│   │   │   └── watermark.rs
│   │   └── verification/    # Data verification
│   │       ├── checksum.rs
│   │       ├── report.rs
│   │       └── verify.rs
│   │
│   ├── adapters/            # External integrations
│   │   ├── openehr/         # OpenEHR adapter
│   │   │   ├── client.rs
│   │   │   ├── models.rs
│   │   │   └── vendor/
│   │   │       ├── trait.rs
│   │   │       ├── ehrbase.rs
│   │   │       └── mod.rs
│   │   └── cosmosdb/        # Cosmos DB adapter
│   │       ├── client.rs
│   │       └── models.rs
│   │
│   ├── domain/              # Domain models
│   │   ├── ids.rs           # Strongly-typed IDs
│   │   ├── composition.rs   # Composition model
│   │   ├── ehr.rs           # EHR model
│   │   └── errors.rs        # Error types
│   │
│   ├── config/              # Configuration
│   │   ├── schema.rs        # Config schema
│   │   └── loader.rs        # Config loading
│   │
│   └── logging/             # Logging
│       ├── structured.rs    # Structured logging
│       └── azure.rs         # Azure integration
│
├── tests/                   # Integration tests
│   ├── integration_test.rs
│   └── config_integration_test.rs
│
├── examples/                # Example configurations
│   └── atlas.example.toml
│
├── docs/                    # Documentation
│   ├── configuration.md
│   ├── architecture.md
│   ├── user-guide.md
│   └── developer-guide.md
│
├── Cargo.toml               # Project manifest
├── Cargo.lock               # Dependency lock file
├── README.md                # Project README
├── CONTRIBUTING.md          # Contribution guidelines
├── CHANGELOG.md             # Change log
└── LICENSE                  # MIT License
```

### Module Responsibilities

- **`cli/`**: User interface, argument parsing, command dispatch
- **`core/`**: Business logic, orchestration, algorithms
- **`adapters/`**: External system integrations (OpenEHR, Cosmos DB)
- **`domain/`**: Core domain types, models, and business rules
- **`config/`**: Configuration loading, validation, and management
- **`logging/`**: Structured logging and observability

## Adding New Features

### Adding a New OpenEHR Vendor

To add support for a new OpenEHR vendor (e.g., Better Platform, Ocean Health):

1. **Create vendor implementation file**:
   ```bash
   touch src/adapters/openehr/vendor/better.rs
   ```

2. **Implement the `OpenEhrVendor` trait**:
   ```rust
   use async_trait::async_trait;
   use crate::adapters::openehr::vendor::OpenEhrVendor;
   use crate::domain::*;

   pub struct BetterPlatformVendor {
       base_url: String,
       client: reqwest::Client,
       config: OpenEhrConfig,
   }

   #[async_trait]
   impl OpenEhrVendor for BetterPlatformVendor {
       async fn get_ehr_ids(&self) -> Result<Vec<EhrId>> {
           // Implementation
       }

       async fn get_compositions_for_ehr(
           &self,
           ehr_id: &EhrId,
           template_id: &TemplateId,
           since: Option<DateTime<Utc>>,
       ) -> Result<Vec<CompositionMetadata>> {
           // Implementation
       }

       async fn fetch_composition(
           &self,
           metadata: &CompositionMetadata,
       ) -> Result<Composition> {
           // Implementation
           // Note: The method accepts CompositionMetadata which contains:
           // - uid: CompositionUid
           // - ehr_id: EhrId
           // - template_id: TemplateId
           // - time_committed: DateTime<Utc>
           // This allows building the correct OpenEHR REST API URL:
           // GET /ehr/{ehr_id}/composition/{uid}
       }

       async fn authenticate(&mut self) -> Result<()> {
           // Implementation
       }
   }
   ```

3. **Update vendor module**:
   ```rust
   // src/adapters/openehr/vendor/mod.rs
   pub mod better;
   ```

4. **Update client factory**:
   ```rust
   // src/adapters/openehr/client.rs
   match config.vendor.as_str() {
       "ehrbase" => Box::new(EhrBaseVendor::new(config)?),
       "better" => Box::new(BetterPlatformVendor::new(config)?),
       _ => return Err(AtlasError::Configuration(...)),
   }
   ```

5. **Add tests**:
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[tokio::test]
       async fn test_better_vendor_creation() {
           // Test implementation
       }
   }
   ```

6. **Update documentation**:
   - Add vendor to configuration guide
   - Update architecture documentation
   - Add example configuration

### Adding a New Transformation Mode

To add a new data transformation mode:

1. **Create transformation file**:
   ```bash
   touch src/core/transform/custom.rs
   ```

2. **Implement transformation function**:
   ```rust
   use crate::domain::*;
   use serde_json::Value;

   pub fn custom_transform(
       composition: &Composition,
       enable_checksum: bool,
   ) -> Result<Value> {
       // Your transformation logic
   }
   ```

3. **Update transform coordinator**:
   ```rust
   // src/core/transform/mod.rs
   pub mod custom;

   pub fn transform_composition(
       composition: &Composition,
       format: &str,
       enable_checksum: bool,
   ) -> Result<Value> {
       match format {
           "preserve" => preserve::preserve_composition(composition, enable_checksum),
           "flatten" => flatten::flatten_composition(composition, enable_checksum),
           "custom" => custom::custom_transform(composition, enable_checksum),
           _ => Err(AtlasError::Configuration(...)),
       }
   }
   ```

4. **Update configuration schema**:
   ```rust
   // src/config/schema.rs
   impl ExportConfig {
       fn validate(&self) -> Result<(), String> {
           let valid_formats = ["preserve", "flatten", "custom"];
           // ...
       }
   }
   ```

5. **Add tests**:
   ```rust
   #[cfg(test)]
   mod tests {
       #[test]
       fn test_custom_transform() {
           // Test implementation
       }
   }
   ```

### Adding a New CLI Command

To add a new CLI command:

1. **Create command file**:
   ```bash
   touch src/cli/commands/mycommand.rs
   ```

2. **Implement command**:
   ```rust
   use clap::Args;
   use crate::config::*;

   #[derive(Debug, Args)]
   pub struct MyCommandArgs {
       /// Command-specific arguments
       #[arg(long)]
       pub my_option: Option<String>,
   }

   pub async fn execute(args: &MyCommandArgs, config: &AtlasConfig) -> Result<()> {
       // Command implementation
       Ok(())
   }
   ```

3. **Update commands module**:
   ```rust
   // src/cli/commands/mod.rs
   pub mod mycommand;
   ```

4. **Add to CLI enum**:
   ```rust
   // src/cli/mod.rs
   #[derive(Debug, Subcommand)]
   pub enum Commands {
       Export(ExportArgs),
       ValidateConfig(ValidateConfigArgs),
       Status(StatusArgs),
       Init(InitArgs),
       MyCommand(MyCommandArgs),  // Add new command
   }
   ```

5. **Add dispatch logic**:
   ```rust
   // src/main.rs
   match cli.command {
       Commands::Export(args) => export::execute(&args, &config).await,
       // ...
       Commands::MyCommand(args) => mycommand::execute(&args, &config).await,
   }
   ```

## Testing Guidelines

### Unit Tests

Place unit tests in the same file as the code being tested:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() {
        // Arrange
        let input = ...;

        // Act
        let result = function_under_test(input);

        // Assert
        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_async_function() {
        // Test async functions
    }
}
```

### Integration Tests

Place integration tests in the `tests/` directory:

```rust
// tests/my_integration_test.rs
use atlas::*;

#[tokio::test]
async fn test_end_to_end_export() {
    // Setup
    let config = load_test_config();

    // Execute
    let result = export::execute(&config).await;

    // Verify
    assert!(result.is_ok());
}
```

### Test Coverage Goals

- **Unit tests**: > 70% code coverage
- **Integration tests**: Cover all major workflows
- **Error paths**: Test error handling and edge cases

### Running Specific Tests

```bash
# Run tests matching pattern
cargo test transform

# Run tests in specific file
cargo test --test integration_test

# Run with specific features
cargo test --features azure
```

## Code Style and Standards

### Rust Guidelines

Atlas follows the [Microsoft Rust Guidelines](https://github.com/microsoft/rust-guidelines):

1. **Error Handling (TR-6.4, TR-6.5, TR-6.6)**:
   - Use `Result<T, E>` for fallible operations
   - Use `thiserror::Error` for domain errors
   - Map external errors to domain errors

2. **Async/Await (TR-6.7)**:
   - Use `async-trait` for async trait methods
   - Use `tokio` runtime for async operations

3. **Documentation (NFR-4.2)**:
   - Document all public items with rustdoc comments
   - Include examples in documentation
   - Document error conditions and panics

### Code Formatting

```bash
# Format all code
cargo fmt

# Check formatting
cargo fmt -- --check
```

### Linting Rules

```bash
# Run clippy with strict warnings
cargo clippy --all-targets -- -D warnings
```

Common clippy rules to follow:
- No `unwrap()` or `expect()` in production code
- Use `?` operator for error propagation
- Avoid unnecessary clones
- Use `&str` instead of `&String` in function parameters

### Naming Conventions

- **Modules**: `snake_case` (e.g., `export_coordinator`)
- **Types**: `PascalCase` (e.g., `ExportCoordinator`)
- **Functions**: `snake_case` (e.g., `execute_export`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `DEFAULT_BATCH_SIZE`)
- **Lifetimes**: Single lowercase letter (e.g., `'a`)

### Documentation Comments

```rust
/// Brief description of the function.
///
/// More detailed explanation if needed.
///
/// # Arguments
///
/// * `arg1` - Description of arg1
/// * `arg2` - Description of arg2
///
/// # Returns
///
/// Description of return value
///
/// # Errors
///
/// This function will return an error if:
/// - Condition 1
/// - Condition 2
///
/// # Examples
///
/// ```
/// use atlas::*;
///
/// let result = my_function(arg1, arg2)?;
/// assert_eq!(result, expected);
/// ```
pub fn my_function(arg1: Type1, arg2: Type2) -> Result<ReturnType> {
    // Implementation
}
```

## Contributing Guidelines

### Before You Start

1. **Check existing issues**: Look for related issues or feature requests
2. **Open a discussion**: For major changes, open an issue first to discuss
3. **Fork the repository**: Create your own fork for development

### Development Workflow

1. **Create a feature branch**:

   ```bash
   git checkout -b feature/my-new-feature
   ```

2. **Make your changes**:
   - Write code following style guidelines
   - Add tests for new functionality
   - Update documentation

3. **Test your changes**:

   ```bash
   cargo test
   cargo clippy --all-targets -- -D warnings
   cargo fmt
   ```

4. **Commit your changes**:

   ```bash
   git add .
   git commit -m "feat: add new feature"
   ```

   Follow [Conventional Commits](https://www.conventionalcommits.org/):
   - `feat:` New feature
   - `fix:` Bug fix
   - `docs:` Documentation changes
   - `test:` Test changes
   - `refactor:` Code refactoring
   - `chore:` Maintenance tasks

5. **Push to your fork**:

   ```bash
   git push origin feature/my-new-feature
   ```

6. **Create a Pull Request**:
   - Provide a clear description of changes
   - Reference related issues
   - Ensure CI passes

### Pull Request Checklist

- [ ] Code follows style guidelines
- [ ] All tests pass
- [ ] New tests added for new functionality
- [ ] Documentation updated
- [ ] CHANGELOG.md updated
- [ ] No clippy warnings
- [ ] Code formatted with `cargo fmt`

## Release Process

### Version Numbering

Atlas follows [Semantic Versioning](https://semver.org/):

- **MAJOR**: Breaking changes
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes (backward compatible)

### Creating a Release

1. **Update version**:

   ```toml
   # Cargo.toml
   [package]
   version = "1.1.0"
   ```

2. **Update CHANGELOG.md**:

   ```markdown
   ## [1.1.0] - 2025-01-15
   ### Added
   - New feature X
   ### Fixed
   - Bug Y
   ```

3. **Commit and tag**:

   ```bash
   git add Cargo.toml CHANGELOG.md
   git commit -m "chore: bump version to 1.1.0"
   git tag -a v1.1.0 -m "Release v1.1.0"
   git push origin main --tags
   ```

4. **Build release binaries**:

   ```bash
   cargo build --release
   ```

5. **Create GitHub release**:

   - Go to GitHub Releases
   - Create new release from tag
   - Upload binaries
   - Copy changelog entry to release notes

---

For more information, see:

- [Configuration Guide](configuration.md) - Configuration reference
- [Architecture Documentation](architecture.md) - System architecture
- [User Guide](user-guide.md) - User documentation
- [CONTRIBUTING.md](../CONTRIBUTING.md) - Contribution guidelines
