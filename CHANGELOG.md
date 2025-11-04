# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
