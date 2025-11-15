# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [2.4.0] - 2025-11-15

### Added

- **Better Platform openEHR Adapter with OIDC Authentication**
  - New vendor implementation: `better` for Better Platform openEHR servers
  - OIDC (OAuth2) authentication support with password grant flow
  - Automatic token refresh with 60-second expiration buffer
  - Thread-safe token management using interior mutability pattern (`Arc<Mutex<TokenState>>`)
  - All openEHR operations implemented using AQL queries:
    - `get_ehr_ids()` - Fetch all EHR IDs from the server
    - `get_compositions_for_ehr()` - Fetch compositions with optional `since` parameter for incremental exports
    - `fetch_composition()` - Fetch individual compositions in FLAT format
  - New configuration fields in `OpenEhrConfig`:
    - `oidc_token_url` - OAuth2 token endpoint URL (required for Better Platform)
    - `client_id` - OAuth2 client ID (required for Better Platform)
  - Environment variable support for OIDC configuration:
    - `ATLAS_OPENEHR_OIDC_TOKEN_URL`
    - `ATLAS_OPENEHR_CLIENT_ID`
  - Example configuration file: `examples/atlas.better.example.toml`
  - Comprehensive documentation in `docs/configuration.md`

### Fixed

- **Better Platform FLAT Format Support**
  - Fixed Accept header for Better Platform composition fetching
  - Better Platform requires custom Accept header: `application/openehr.wt.flat+json`
  - Previously was returning XML instead of JSON due to incorrect header
  - Added documentation note about Better Platform's custom Accept headers

- **Capitalization Consistency**
  - Fixed all instances of 'OpenEHR' to 'openEHR' (lowercase 'o') throughout codebase
  - Updated 107 instances across 34 Rust source files
  - Includes error messages, comments, documentation, and code
  - Ensures consistent branding and terminology

### Changed

- **openEHR Client Factory**
  - Updated `OpenEhrClient::new()` to support `better` vendor type
  - Enhanced error messages for unsupported vendors

### Testing

- All 218 unit tests + 69 doc tests passing
- Strict clippy linting passing (`--all-targets --all-features -- -D warnings -D clippy::all`)
- Successfully tested against Better Platform sandbox environment
- Data successfully exported to Azure Cosmos DB

### Documentation

- Updated `README.md` with Better Platform support information
- Updated `docs/configuration.md` with Better Platform configuration guide
- Updated `examples/README.md` with Better Platform example
- Added note about custom Accept headers used by Better Platform
- Fixed capitalization throughout all documentation files

## [2.3.0] - 2025-11-12

### Added

- **GDPR/HIPAA Anonymization Support** (Phase 1)
  - Automatic PII detection for 18 HIPAA Safe Harbor identifiers
  - Detection of 6 GDPR quasi-identifiers (occupation, education, marital status, ethnicity, age, gender)
  - 50+ regex patterns for comprehensive coverage with confidence scoring
  - Flexible anonymization strategies: Token (unique random tokens) and Redact (category markers)
  - Compliance modes: HIPAA Safe Harbor and GDPR
  - Dry-run mode for previewing PII detections without anonymizing data
  - Comprehensive audit logging with SHA-256 hashed PII values (never logs plaintext)
  - JSON and plain text audit log formats with tamper-evident trail
  - 12-factor configuration support (TOML, environment variables, CLI flags)
  - New CLI flags: `--anonymize`, `--anonymize-mode`, `--anonymize-dry-run`
  - New environment variables for anonymization configuration
  - Integration into export pipeline via `BatchProcessor::process_batch()`
  - New `DatabaseClient::bulk_insert_json()` method for pre-transformed JSON documents
  - Comprehensive user guide: `docs/anonymization-user-guide.md`
  - Manual testing guide with 6 validation scenarios
  - Enhanced rustdoc comments for all anonymization APIs

### Changed

- **Database Client Interface Enhancement**
  - Added `bulk_insert_json()` method to `DatabaseClient` trait
  - Implemented in `CosmosDbAdapter` and `PostgreSqlAdapter`
  - Enables clean anonymization integration between transformation and database write

- **Export Pipeline Integration**
  - Anonymization integrated into batch processing workflow
  - New `transform_and_anonymize()` method for composition processing
  - Anonymization statistics tracked in `BatchResult`

### Fixed

- **Documentation Improvements**
  - Removed `.await` from non-async doctest examples
  - Corrected TOML enum values to snake_case for anonymization config
  - Fixed manual testing guide with correct CLI usage
  - Minor documentation updates to README.md

- **Code Quality**
  - Resolved all clippy warnings in anonymization module
  - Removed unnecessary async from anonymization functions
  - Improved code consistency and quality

### Testing

- Added 207 unit tests (24 anonymization-specific)
- Added 41 integration tests with synthetic OpenEHR data
- Added 56 compliance tests (HIPAA + GDPR validation)
- Added 56 doctests for API examples
- Total: 304 tests passing

### Dependencies

- Added `fancy-regex = "0.13"` - Advanced regex patterns with lookahead/lookbehind
- Added `rand = "0.8"` - Random token generation
- Added `sha2 = "0.10"` - SHA-256 hashing for audit logs
- Added `fake = "2.9"` - Test fixture generation (dev-only)

### Known Limitations (Phase 1)

- Free-text anonymization uses basic pattern matching only (transformer-based NER planned for Phase 2)
- Language support: English only (multi-language support planned for Phase 2)
- k-anonymity verification not yet implemented (planned for Phase 2)
- Formal performance benchmarks deferred to future release

## [2.2.0] - 2025-11-10

### Added

- **Environment-Based TLS Verification Enforcement** (Feature #13, PR #11)
  - Added `environment` configuration field with three values: `development`, `staging`, `production`
  - Runtime WARN level logging when TLS certificate verification is disabled in any environment
  - Production environments now block TLS verification from being disabled (validation error)
  - Three-tier security approach: development/staging (permissive with warnings), production (strict enforcement)
  - Updated all documentation with security warnings and best practices
  - Added comprehensive tests for environment-aware validation with mutex synchronization

- **12-Factor App Compliance with Environment Variable Support** (Feature #34, PR #10)
  - Complete environment variable override support for all configuration fields
  - Added `ATLAS_DATABASE_TARGET` for runtime database backend selection (CosmosDB/PostgreSQL)
  - Array support in environment variables (JSON or comma-separated format)
  - Empty string clears array values
  - Smart validation that only requires CosmosDB config when CosmosDB is the target database
  - Comprehensive documentation with environment variable reference table
  - 40+ new environment variables for complete configuration override capability

### Changed

- **Configuration Schema Simplification** (PR #10)
  - Removed `application.name` and `application.version` fields from configuration schema
  - Application name and version now come from `Cargo.toml` at compile time
  - Users must remove these two fields from existing `atlas.toml` files
  - Updated `validate-config` command to use compile-time package metadata
  - Updated all example configurations and documentation

### Fixed

- **Clippy Warnings** (commit be1bdbe)
  - Use derive attribute for `Environment` Default implementation
  - Improved code quality and consistency

- **Release Workflow** (PR #9)
  - Fixed GitHub Actions release workflow to support immutable releases
  - Implemented Draft → Upload → Publish pattern
  - Releases are now created as drafts, assets uploaded, then published
  - Added `allow-missing-changelog` option for test releases
  - No more HTTP 422 errors when uploading assets

## [2.1.0] - 2025-11-09

### Added

- **Standardized Error Handling** (Feature #29)
  - Created `ResultExt` trait for adding context to errors
  - Added `.context()` and `.with_context()` methods similar to anyhow
  - Maintains `Result<T, AtlasError>` for library code
  - Keeps `anyhow::Result` only in CLI layer
  - Added comprehensive error handling guidelines to CONTRIBUTING.md
  - Added 6 new unit tests for error conversions

- **Secure Credential Handling** (Feature #11)
  - All credentials now wrapped in `Secret<SecretValue>` type with automatic memory zeroization
  - Debug output shows `Secret([REDACTED])` instead of actual values
  - Credentials never appear in logs or error messages
  - Explicit `expose_secret()` calls required for access (easy to audit)
  - Protected credentials: OpenEHR password, Cosmos DB key, PostgreSQL connection string, Azure client secret
  - Added `secrecy` (0.8) and `zeroize` (1.8) dependencies

- **Complete Dry-Run Validation** (Feature #7)
  - Added `dry_run` field to `ExportConfig` and `BatchConfig`
  - Skip all database writes (compositions and watermarks) in dry-run mode
  - Added `dry_run` field to `ExportSummary` with clear logging
  - Added 14 comprehensive integration tests for dry-run mode
  - Updated user guide with dedicated dry-run section

- **Graceful Shutdown Handling** (Feature #2)
  - Added SIGTERM and SIGINT signal handlers
  - Created shutdown signal channel using `tokio::sync::watch`
  - Check shutdown signal between templates and EHRs
  - Added `Interrupted` status to `ExportStatus` enum
  - Added `interrupted` flag and `shutdown_reason` to `ExportSummary`
  - Configurable shutdown timeout (default: 30s via `shutdown_timeout_secs`)
  - Return exit code 130 for interrupted exports
  - Complete current batch before shutdown (no mid-batch interruption)
  - Added 12 comprehensive integration tests for graceful shutdown
  - Documentation for Docker and Kubernetes graceful shutdown best practices

- **Comprehensive Unit Tests**
  - Added mock implementations for `DatabaseClient` and `StateStorage`
  - Added 6 new unit tests for `BatchProcessor` covering empty compositions, successful processing (preserve/flatten), failures, dry-run mode, and watermark updates
  - Added 9 baseline unit tests for `ExportCoordinator` covering shutdown signals and basic functionality

- **Security Policy**
  - Added `SECURITY.md` with vulnerability reporting guidelines
  - Documented security best practices for credential handling

### Changed

- **Large Functions Refactoring** (Feature #30)
  - Refactored `execute_export` from 176 lines to 39 lines (now acts as clean orchestrator)
  - Created `validate_and_prepare_export()` helper for config validation
  - Created `process_templates()` and `process_ehrs_for_template()` helpers for iteration logic
  - Created `run_post_export_verification()` helper for verification logic
  - Refactored `process_ehr_for_template` from 135 lines to 54 lines
  - Created `load_or_create_watermark()`, `fetch_compositions_for_ehr()`, and `process_and_update_summary()` helpers
  - All helper functions are under 50 lines
  - All coordinator and batch processor tests pass

- **Documentation Improvements**
  - Cleanup and architecture updates
  - Removed SHA-256 checksum references from README.md
  - Updated architecture diagrams to show both Cosmos DB and PostgreSQL backends
  - Cleaned up Future Enhancements section
  - Updated `docs/README.md` with hyperlinks and better organization
  - Updated `docs/architecture.md` to match dual-backend architecture
  - Enhanced `docs/configuration.md` with credential protection section
  - Updated `docs/user-guide.md` security best practices
  - Expanded README.md security section

### Fixed

- Updated all 53 failing doctests to match current API
  - Fixed config loading from `AtlasConfig::from_file()` to `load_config()`
  - Fixed `init_logging()` calls to include both `log_level_str` and config parameters
  - Updated `StateManager::new()` to `StateManager::new_with_storage()`
  - Fixed transform function signatures to use `String` for `export_mode`
  - Fixed `SecretString` usage to use `Secret::new(SecretValue::from(...))`
  - Fixed ID type conversions to use proper error handling
  - Updated field names in `ExportSummary`
  - Re-enabled doctests in CI workflow

- Updated integration tests for secure credential handling
  - Fixed password assertions to use `expose_secret().as_ref()`
  - Updated `azure_client_secret` to use `Secret::new(SecretValue::from(...))`

- Resolved clippy warnings for boolean assertions
  - Replaced `assert_eq!(bool, false)` with `assert!(!bool)`

### Removed

- **Checksum Verification Complexity**
  - Removed checksum calculation and validation from verification process
  - Simplified verification to existence-only checks
  - Deleted `src/core/verification/checksum.rs` file
  - Removed `checksum_algorithm` from `VerificationConfig`
  - Removed `checksum` field from `ExportedCompositionInfo`
  - Removed `enable_checksum` parameter from transform functions and database methods
  - Removed 8 checksum-related tests
  - Updated documentation to reflect existence-based verification

- Removed test configuration files (`test-minimal.toml`, `test-examples.toml`)

## [2.0.0] - 2025-11-08

### Added

- PostgreSQL backend support as alternative to Cosmos DB
- Dual-backend architecture with database abstraction layer
- Complete migration system for PostgreSQL schema management
- Connection pooling for PostgreSQL using `deadpool-postgres`
- Comprehensive PostgreSQL adapter with full feature parity
- Example configurations for PostgreSQL deployments

### Changed

- Refactored database layer to support multiple backends
- Updated configuration schema to support database target selection
- Enhanced documentation for multi-backend deployment scenarios

## [1.0.0] - 2025-11-04

### Added

- Azure Log Analytics integration using modern Logs Ingestion API
  - Azure AD authentication with ClientSecretCredential
  - Support for Data Collection Rules (DCR) and Data Collection Endpoints (DCE)
  - Structured logging for export operations, errors, and performance metrics
  - Comprehensive Azure Portal setup guide (`docs/azure-log-analytics-setup.md`)
- New configuration fields for Azure logging:
  - `azure_tenant_id`, `azure_client_id`, `azure_client_secret`
  - `azure_dcr_immutable_id`, `azure_dce_endpoint`, `azure_stream_name`
- Initial project scaffolding
- Directory structure following PRD architecture
- Cargo.toml with all required dependencies
- MIT License
- README.md with project overview
- CONTRIBUTING.md with development guidelines
- Example configuration file
- Basic test infrastructure
- `.env` file to `.gitignore` for environment variable protection

### Changed

- Updated Azure SDK dependencies to compatible versions:
  - `azure_core` to 0.29.1
  - `azure_identity` to 0.29
- Updated configuration schema to support modern Azure authentication
- Updated all example configuration files with new Azure logging fields
- Added "Erik Howard" as project author in `Cargo.toml`
- Added "healthcare" category to `Cargo.toml`

### Removed

- Application Insights integration (replaced with Azure Log Analytics)
- Deprecated HTTP Data Collector API support (replaced with Logs Ingestion API)
- Legacy shared key authentication for Azure logging
