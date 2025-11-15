# Atlas Configuration Guide

This guide provides comprehensive documentation for all Atlas configuration options.

## Table of Contents

- [Atlas Configuration Guide](#atlas-configuration-guide)
  - [Table of Contents](#table-of-contents)
  - [Configuration File Format](#configuration-file-format)
  - [Environment Variable Substitution](#environment-variable-substitution)
  - [Configuration Sections](#configuration-sections)
    - [Application](#application)
    - [OpenEHR](#openehr)
      - [OpenEHR Retry Configuration](#openehr-retry-configuration)
      - [OpenEHR Query Configuration](#openehr-query-configuration)
    - [Export](#export)
    - [Cosmos DB](#cosmos-db)
    - [PostgreSQL](#postgresql)
    - [State Management](#state-management)
    - [Verification](#verification)
    - [Logging](#logging)
  - [Complete Example](#complete-example)
  - [Common Configurations](#common-configurations)
    - [Clinical Research Export](#clinical-research-export)
    - [Incremental Daily Sync](#incremental-daily-sync)
    - [ML Feature Extraction](#ml-feature-extraction)
    - [Development/Testing](#developmenttesting)
  - [Validation](#validation)
  - [Security Best Practices](#security-best-practices)
  - [Troubleshooting](#troubleshooting)
    - [Configuration Not Found](#configuration-not-found)
    - [Invalid Configuration](#invalid-configuration)
    - [Environment Variable Not Substituted](#environment-variable-not-substituted)
    - [Connection Failures](#connection-failures)
    - [TLS Certificate Verification Errors](#tls-certificate-verification-errors)

## Configuration File Format

Atlas uses TOML (Tom's Obvious, Minimal Language) for configuration. The default configuration file is `atlas.toml` in the current directory.

To generate a sample configuration:

```bash
# Generate minimal configuration
atlas init

# Generate configuration with examples and comments
atlas init --with-examples
```

To validate your configuration:

```bash
atlas validate-config -c atlas.toml
```

## Environment Variable Substitution

Atlas supports environment variable substitution in configuration values using the `${VAR_NAME}` syntax. This is useful for sensitive values like passwords and API keys.

### Automatic .env File Loading

Atlas automatically loads environment variables from a `.env` file in the project root directory when it starts. This makes it easy to manage credentials without setting them manually in your shell.

**Example .env file:**

```bash
# .env
ATLAS_OPENEHR_USERNAME=ehrbaseuser
ATLAS_OPENEHR_PASSWORD=ehrbasepassword
ATLAS_PG_PASSWORD=your_postgres_password
```

**Important Notes:**
- The `.env` file is loaded automatically - no additional configuration needed
- If a `.env` file doesn't exist, Atlas will use environment variables from your shell
- Environment variables set in your shell take precedence over `.env` file values
- **Never commit your `.env` file to version control** - add it to `.gitignore`
- Use `.env.example` as a template (copy it to `.env` and fill in your values)

### Configuration File Usage

Example configuration using environment variables:

```toml
username = "${ATLAS_OPENEHR_USERNAME}"
password = "${ATLAS_OPENEHR_PASSWORD}"
key = "${ATLAS_COSMOSDB_KEY}"
```

**Database-Specific Variables:**
- Only set environment variables for the database you're actually using
- If `database_target = "postgresql"`, you don't need `ATLAS_COSMOSDB_KEY`
- If `database_target = "cosmosdb"`, you don't need `ATLAS_PG_PASSWORD`

## Configuration Sections

### Application

Application-level settings that control Atlas behavior.

```toml
[application]
log_level = "info"

# Runtime environment (development, staging, production)
environment = "development"
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `log_level` | string | "info" | Log verbosity: `trace`, `debug`, `info`, `warn`, `error` |

### Environment

Runtime environment configuration that affects security policies and validation behavior.

```toml
# Runtime environment
environment = "production"  # Options: development, staging, production
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `environment` | string | "development" | Runtime environment: `development`, `staging`, or `production` |

**Environment Variable Override**: `ATLAS_ENVIRONMENT`

**Important**: The environment setting affects security validation:
- **Production**: TLS certificate verification MUST be enabled (`tls_verify = true`). Configuration validation will fail if TLS verification is disabled.
- **Staging**: TLS verification can be disabled with a warning logged at startup.
- **Development**: TLS verification can be disabled with a warning logged at startup.

**Example**:
```bash
# Set environment via environment variable
export ATLAS_ENVIRONMENT=production
atlas export
```

### OpenEHR

Configuration for connecting to OpenEHR servers. Atlas supports multiple vendor implementations including EHRBase and Better Platform.

#### EHRBase Configuration

```toml
[openehr]
base_url = "https://ehrbase.example.com/ehrbase"
vendor_type = "ehrbase"
auth_type = "basic"
username = "${ATLAS_OPENEHR_USERNAME}"
password = "${ATLAS_OPENEHR_PASSWORD}"
tls_verify = true
tls_verify_certificates = true
timeout_seconds = 60
```

#### Better Platform Configuration

```toml
[openehr]
base_url = "https://sandbox.better.care/ehr"
vendor_type = "better"
username = "${ATLAS_OPENEHR_USERNAME}"
password = "${ATLAS_OPENEHR_PASSWORD}"
oidc_token_url = "https://sandbox.better.care/auth/realms/portal/protocol/openid-connect/token"
client_id = "portal"
tls_verify = true
tls_verify_certificates = true
timeout_seconds = 60
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `base_url` | string | **required** | Base URL of the OpenEHR server. For EHRBase: `https://ehrbase.example.com/ehrbase`. For Better: `https://sandbox.better.care/ehr`. Do not include `/rest/openehr/v1` - Atlas adds this automatically. |
| `vendor_type` | string | "ehrbase" | Vendor implementation: `ehrbase` or `better` |
| `auth_type` | string | "basic" | Authentication type: `basic` (used for both EHRBase and Better OIDC) |
| `username` | string | null | Username for authentication (required) |
| `password` | string | null | Password for authentication (required) |
| `oidc_token_url` | string | null | OIDC token endpoint URL (required for Better Platform, e.g., `https://sandbox.better.care/auth/realms/portal/protocol/openid-connect/token`) |
| `client_id` | string | null | OIDC client ID (required for Better Platform, e.g., `portal`) |
| `tls_verify` | boolean | true | Enable TLS certificate verification (alias for `tls_verify_certificates`) |
| `tls_verify_certificates` | boolean | true | Enable TLS certificate verification (alias for `tls_verify`) |
| `tls_ca_cert` | string | null | Optional path to custom CA certificate file |
| `timeout_seconds` | integer | 60 | Request timeout in seconds |

**Vendor-Specific Notes:**

- **EHRBase**: Uses HTTP Basic Authentication. Only requires `username` and `password`.
- **Better Platform**: Uses OIDC (OAuth2) with password grant flow. Requires `username`, `password`, `oidc_token_url`, and `client_id`. Tokens are automatically refreshed when they expire.

**⚠️ CRITICAL SECURITY WARNING - TLS Certificate Verification:**

**Production Enforcement**: When `environment = "production"`, TLS certificate verification **CANNOT** be disabled. Configuration validation will fail with an error if `tls_verify = false` or `tls_verify_certificates = false`.

**Security Implications**:
- Disabling TLS verification exposes your application to **man-in-the-middle (MITM) attacks**
- Attackers can intercept, read, and modify sensitive health data in transit
- This violates HIPAA, GDPR, and other healthcare data protection regulations
- **NEVER disable TLS verification in production environments**

**Configuration Guidelines**:

1. **Production (Recommended)**: Use trusted CA certificates
   ```toml
   environment = "production"
   tls_verify = true  # Required - cannot be disabled
   ```

2. **Production with Self-Signed Certificates**: Use custom CA certificate
   ```toml
   environment = "production"
   tls_verify = true
   tls_ca_cert = "/path/to/your-ca-certificate.pem"
   ```

3. **Development/Testing Only**: Can disable with warning
   ```toml
   environment = "development"  # or "staging"
   tls_verify = false  # ⚠️ Logs security warning at startup
   ```

**Notes**:
- Both `tls_verify` and `tls_verify_certificates` control the same setting - use either one
- A runtime warning is logged at WARN level whenever TLS verification is disabled
- For containerized deployments, use `ATLAS_ENVIRONMENT=production` to enforce security policies

#### OpenEHR Retry Configuration

```toml
[openehr.retry]
max_retries = 3
initial_delay_ms = 1000
max_delay_ms = 30000
backoff_multiplier = 2.0
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `max_retries` | integer | 3 | Maximum number of retry attempts for failed requests |
| `initial_delay_ms` | integer | 1000 | Initial delay in milliseconds before first retry |
| `max_delay_ms` | integer | 30000 | Maximum delay in milliseconds between retries |
| `backoff_multiplier` | float | 2.0 | Multiplier for exponential backoff (delay *= multiplier) |

#### OpenEHR Query Configuration

```toml
[openehr.query]
template_ids = ["IDCR - Adverse Reaction List.v1", "IDCR - Problem List.v1"]
ehr_ids = []
time_range_start = "2024-01-01T00:00:00Z"
time_range_end = null
batch_size = 1000
parallel_ehrs = 8
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `template_ids` | array[string] | **required** | List of OpenEHR template IDs to export (cannot be empty) |
| `ehr_ids` | array[string] | [] | List of specific EHR IDs to export (empty = all EHRs) |
| `time_range_start` | string | null | Start of time range filter (ISO 8601 format, e.g., "2024-01-01T00:00:00Z") |
| `time_range_end` | string | null | End of time range filter (ISO 8601 format, null = now) |
| `batch_size` | integer | 1000 | Number of compositions to process per batch (100-5000) |
| `parallel_ehrs` | integer | 8 | Number of EHRs to process concurrently (1-100) |

### Export

Export behavior and data transformation settings.

```toml
[export]
mode = "incremental"
export_composition_format = "preserve"
database_target = "cosmosdb"  # or "postgresql"
max_retries = 3
retry_backoff_ms = [1000, 2000, 4000]
shutdown_timeout_secs = 30
dry_run = false
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `mode` | string | "incremental" | Export mode: `full` (all data) or `incremental` (only new/changed data since last export) |
| `export_composition_format` | string | "preserve" | Data format: `preserve` (exact FLAT JSON structure) or `flatten` (convert paths to field names) |
| `database_target` | string | **required** | Database backend: `cosmosdb` or `postgresql` |
| `max_retries` | integer | 3 | Maximum retry attempts for failed exports (0-10) |
| `retry_backoff_ms` | array[integer] | [1000, 2000, 4000] | Retry delay intervals in milliseconds |
| `shutdown_timeout_secs` | integer | 30 | Graceful shutdown timeout in seconds. Maximum time to wait for current batch to complete when SIGTERM/SIGINT is received. Should align with container orchestration grace periods (e.g., Kubernetes default is 30s) |
| `dry_run` | boolean | false | Dry-run mode - simulate export without writing to database. When enabled, all database write operations (compositions and watermarks) are skipped, but the export process runs normally. Useful for testing configuration and previewing what would be exported. Can also be enabled via `--dry-run` CLI flag |

**Export Modes:**

- **`full`**: Exports all compositions matching the query, regardless of previous exports
- **`incremental`**: Uses watermark tracking to export only compositions created/modified since the last successful export

**Composition Formats:**

- **`preserve`**: Maintains the exact FLAT JSON structure from OpenEHR in the `content` field
- **`flatten`**: Converts OpenEHR path notation (e.g., `vital_signs/blood_pressure/systolic`) to flat field names (e.g., `vital_signs_blood_pressure_systolic`)

### Cosmos DB

Azure Cosmos DB connection and container settings.

```toml
[cosmosdb]
endpoint = "https://myaccount.documents.azure.com:443/"
key = "${ATLAS_COSMOS_KEY}"
database_name = "openehr_data"
control_container = "atlas_control"
data_container_prefix = "compositions"
partition_key = "/ehr_id"
max_concurrency = 10
request_timeout_seconds = 60
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `endpoint` | string | **required** | Cosmos DB account endpoint URL (must start with https://) |
| `key` | string | **required** | Cosmos DB primary or secondary access key |
| `database_name` | string | **required** | Name of the Cosmos DB database |
| `control_container` | string | "atlas_control" | Container name for Atlas state/watermark storage |
| `data_container_prefix` | string | "compositions" | Prefix for data containers (results in `{prefix}_{template_id}`) |
| `partition_key` | string | "/ehr_id" | Partition key path for data containers (recommended: `/ehr_id`) |
| `max_concurrency` | integer | 10 | Maximum concurrent operations to Cosmos DB (1-100) |
| `request_timeout_seconds` | integer | 60 | Request timeout in seconds |

**Container Naming:**

- Control container: Uses the exact name specified in `control_container`
- Data containers: Named as `{data_container_prefix}_{template_id}` (e.g., `compositions_IDCR - Adverse Reaction List.v1`)

**Partition Key:**

- Must be `/ehr_id` for optimal patient-based queries
- Ensures all compositions for a patient are co-located

### PostgreSQL

PostgreSQL database connection and configuration (alternative to Cosmos DB).

```toml
[postgresql]
connection_string = "postgresql://atlas_user:${ATLAS_PG_PASSWORD}@localhost:5432/openehr_data?sslmode=require"
max_connections = 20
connection_timeout_seconds = 30
statement_timeout_seconds = 60
ssl_mode = "require"
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `connection_string` | string | **required** | PostgreSQL connection string (supports environment variable substitution) |
| `max_connections` | integer | 20 | Maximum connections in the connection pool (1-100) |
| `connection_timeout_seconds` | integer | 30 | Timeout for acquiring a connection from the pool |
| `statement_timeout_seconds` | integer | 60 | Timeout for executing SQL statements |
| `ssl_mode` | string | "require" | SSL/TLS mode: `disable`, `allow`, `prefer`, `require`, `verify-ca`, `verify-full` |

**Connection String Format:**

```
postgresql://[user[:password]@][host][:port][/dbname][?param1=value1&...]
```

**Examples:**

```toml
# Local development
connection_string = "postgresql://atlas_user:password@localhost:5432/openehr_data"

# Production with SSL
connection_string = "postgresql://atlas_user:${ATLAS_PG_PASSWORD}@db.example.com:5432/openehr_data?sslmode=require"

# Azure Database for PostgreSQL
connection_string = "postgresql://atlas_user@myserver:${ATLAS_PG_PASSWORD}@myserver.postgres.database.azure.com:5432/openehr_data?sslmode=require"
```

**SSL Modes:**

- `disable`: No SSL (development only)
- `require`: Require SSL without certificate verification (minimum for production)
- `verify-ca`: Require SSL and verify CA certificate (recommended for production)
- `verify-full`: Require SSL, verify CA, and verify hostname (most secure)

**Database Setup:**

Before using PostgreSQL, you must create the database schema. See the [PostgreSQL Setup Guide](postgresql-setup.md) for detailed instructions.

```bash
# Run the migration script
psql -U atlas_user -d openehr_data -f migrations/001_initial_schema.sql
```

### State Management

Watermark and checkpoint configuration for incremental exports.

```toml
[state]
enable_checkpointing = true
checkpoint_interval_seconds = 30
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enable_checkpointing` | boolean | true | Enable automatic watermark checkpointing during export |
| `checkpoint_interval_seconds` | integer | 30 | Interval in seconds between checkpoint saves (must be > 0) |

**How Checkpointing Works:**

- Atlas tracks the last successfully exported composition timestamp per {template_id, ehr_id} combination
- Checkpoints are saved to the control container after each batch
- If export fails, Atlas resumes from the last checkpoint on next run
- Disable checkpointing only for testing or one-time full exports

### Verification

Optional post-export verification to ensure exported compositions exist in the database.

```toml
[verification]
enable_verification = false
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enable_verification` | boolean | false | Enable post-export data verification |

**Verification Process:**

When enabled, Atlas performs the following verification steps:

1. **After Export**: Fetches each exported composition from the database to verify it exists
2. **Reporting**: Generates a detailed verification report showing pass/fail status for each composition

**Verification Report Includes:**

- Total compositions verified
- Number passed/failed/skipped
- Success rate percentage
- Detailed failure information (composition UID, reason)
- Verification duration

**Important Notes:**

- Verification is currently only available when using **Azure Cosmos DB** as the database target
- Verification adds overhead to the export process (typically 10-20% longer)
- Recommended for critical data exports where you want to confirm all compositions were successfully written
- Failed verifications indicate compositions that were not found in the database

### Logging

Logging configuration for local files and Azure integration.

```toml
[logging]
local_enabled = true
local_path = "/var/log/atlas"
local_rotation = "daily"
local_max_size_mb = 100
azure_enabled = false
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `local_enabled` | boolean | true | Enable local file logging |
| `local_path` | string | "/var/log/atlas" | Directory path for log files |
| `local_rotation` | string | "daily" | Log rotation strategy: `daily` or `size` |
| `local_max_size_mb` | integer | 100 | Maximum log file size in MB (for size-based rotation) |
| `azure_enabled` | boolean | false | Enable Azure Log Analytics logging |
| `azure_tenant_id` | string | null | Azure AD tenant ID |
| `azure_client_id` | string | null | Azure AD client ID (from App Registration) |
| `azure_client_secret` | string | null | Azure AD client secret |
| `azure_log_analytics_workspace_id` | string | null | Log Analytics workspace ID |
| `azure_dcr_immutable_id` | string | null | Data Collection Rule immutable ID |
| `azure_dce_endpoint` | string | null | Data Collection Endpoint URL |
| `azure_stream_name` | string | null | Stream name (e.g., "Custom-AtlasExport_CL") |

**Azure Log Analytics:**

- Uses modern Logs Ingestion API with Azure AD authentication
- Requires Azure AD App Registration and Data Collection Rule (DCR)
- Logs export operations, errors, and performance metrics
- Recommended for production deployments
- See Azure setup guide for detailed configuration instructions

## Environment Variable Overrides

Atlas supports comprehensive environment variable overrides for all configuration options, enabling 12-factor app compliance and containerized deployments.

### Overview

All configuration values can be overridden using environment variables with the `ATLAS_<SECTION>_<KEY>` pattern. Environment variable overrides take precedence over TOML file values.

### Array Format Support

Array fields support both JSON and comma-separated formats:

```bash
# JSON format (recommended for complex values with spaces or special characters)
export ATLAS_OPENEHR_QUERY_TEMPLATE_IDS='["IDCR - Vital Signs.v1","IDCR - Lab Report.v1"]'

# Comma-separated format (simpler for basic values)
export ATLAS_OPENEHR_QUERY_EHR_IDS="ehr-123,ehr-456,ehr-789"

# Numeric arrays
export ATLAS_EXPORT_RETRY_BACKOFF_MS="1000,2000,4000"
export ATLAS_EXPORT_RETRY_BACKOFF_MS='[1000,2000,4000]'  # JSON also works

# Empty string clears the array
export ATLAS_OPENEHR_QUERY_EHR_IDS=""
```

### Complete Environment Variable Reference

#### Database Selection

| Environment Variable | Type | Description | Example |
|---------------------|------|-------------|---------|
| `ATLAS_ENVIRONMENT` | string | Runtime environment: `development`, `staging`, `production` | `production` |
| `ATLAS_DATABASE_TARGET` | string | Database target: `cosmosdb` or `postgresql` | `postgresql` |

#### Application

| Environment Variable | Type | Description | Example |
|---------------------|------|-------------|---------|
| `ATLAS_APPLICATION_LOG_LEVEL` | string | Log level: `trace`, `debug`, `info`, `warn`, `error` | `debug` |
| `ATLAS_APPLICATION_DRY_RUN` | boolean | Dry run mode (no database writes) | `true` |

#### OpenEHR Connection

| Environment Variable | Type | Description | Example |
|---------------------|------|-------------|---------|
| `ATLAS_OPENEHR_BASE_URL` | string | OpenEHR server base URL | `https://ehrbase.example.com` |
| `ATLAS_OPENEHR_USERNAME` | string | OpenEHR username | `atlas_user` |
| `ATLAS_OPENEHR_PASSWORD` | string | OpenEHR password (sensitive) | `secret` |
| `ATLAS_OPENEHR_VENDOR_TYPE` | string | OpenEHR vendor: `ehrbase`, `better` | `ehrbase` |
| `ATLAS_OPENEHR_AUTH_TYPE` | string | Authentication type: `basic` | `basic` |
| `ATLAS_OPENEHR_OIDC_TOKEN_URL` | string | OIDC token endpoint (Better Platform only) | `https://sandbox.better.care/auth/realms/portal/protocol/openid-connect/token` |
| `ATLAS_OPENEHR_CLIENT_ID` | string | OIDC client ID (Better Platform only) | `portal` |
| `ATLAS_OPENEHR_TLS_VERIFY` | boolean | Enable TLS verification | `true` |
| `ATLAS_OPENEHR_TLS_VERIFY_CERTIFICATES` | boolean | Verify TLS certificates | `true` |
| `ATLAS_OPENEHR_TLS_CA_CERT` | string | Path to custom CA certificate | `/path/to/ca.pem` |
| `ATLAS_OPENEHR_TIMEOUT_SECONDS` | integer | Request timeout in seconds | `120` |

#### OpenEHR Retry

| Environment Variable | Type | Description | Example |
|---------------------|------|-------------|---------|
| `ATLAS_OPENEHR_RETRY_MAX_RETRIES` | integer | Maximum retry attempts (0-10) | `5` |
| `ATLAS_OPENEHR_RETRY_INITIAL_DELAY_MS` | integer | Initial retry delay in milliseconds | `2000` |
| `ATLAS_OPENEHR_RETRY_MAX_DELAY_MS` | integer | Maximum retry delay in milliseconds | `60000` |
| `ATLAS_OPENEHR_RETRY_BACKOFF_MULTIPLIER` | float | Retry backoff multiplier | `2.5` |

#### Query Configuration

| Environment Variable | Type | Description | Example |
|---------------------|------|-------------|---------|
| `ATLAS_OPENEHR_QUERY_TEMPLATE_IDS` | array | Template IDs to query (JSON or CSV) | `["IDCR - Vital Signs.v1"]` |
| `ATLAS_OPENEHR_QUERY_EHR_IDS` | array | Specific EHR IDs to query (JSON or CSV) | `ehr-123,ehr-456` |
| `ATLAS_OPENEHR_QUERY_TIME_RANGE_START` | string | Query time range start (ISO 8601) | `2024-01-01T00:00:00Z` |
| `ATLAS_OPENEHR_QUERY_TIME_RANGE_END` | string | Query time range end (ISO 8601) | `2024-12-31T23:59:59Z` |
| `ATLAS_OPENEHR_QUERY_BATCH_SIZE` | integer | Query batch size (100-5000) | `2000` |
| `ATLAS_OPENEHR_QUERY_PARALLEL_EHRS` | integer | Parallel EHR processing (1-100) | `16` |

#### Export Configuration

| Environment Variable | Type | Description | Example |
|---------------------|------|-------------|---------|
| `ATLAS_EXPORT_MODE` | string | Export mode: `full` or `incremental` | `incremental` |
| `ATLAS_EXPORT_COMPOSITION_FORMAT` | string | Composition format: `preserve` or `flatten` | `flatten` |
| `ATLAS_EXPORT_MAX_RETRIES` | integer | Maximum export retries (0-10) | `5` |
| `ATLAS_EXPORT_RETRY_BACKOFF_MS` | array | Retry backoff delays in ms (JSON or CSV) | `1000,2000,4000` |
| `ATLAS_EXPORT_SHUTDOWN_TIMEOUT_SECS` | integer | Shutdown timeout in seconds | `60` |
| `ATLAS_EXPORT_DRY_RUN` | boolean | Export dry run mode | `false` |

#### Cosmos DB

| Environment Variable | Type | Description | Example |
|---------------------|------|-------------|---------|
| `ATLAS_COSMOSDB_ENDPOINT` | string | Cosmos DB endpoint URL | `https://myaccount.documents.azure.com:443/` |
| `ATLAS_COSMOSDB_KEY` | string | Cosmos DB access key (sensitive) | `secret-key` |
| `ATLAS_COSMOSDB_DATABASE_NAME` | string | Cosmos DB database name | `openehr_data` |
| `ATLAS_COSMOSDB_CONTROL_CONTAINER` | string | Control container name | `atlas_control` |
| `ATLAS_COSMOSDB_DATA_CONTAINER_PREFIX` | string | Data container prefix | `compositions` |
| `ATLAS_COSMOSDB_PARTITION_KEY` | string | Partition key path | `/ehr_id` |
| `ATLAS_COSMOSDB_MAX_CONCURRENCY` | integer | Maximum concurrent operations | `20` |
| `ATLAS_COSMOSDB_REQUEST_TIMEOUT_SECONDS` | integer | Request timeout in seconds | `90` |

#### PostgreSQL

| Environment Variable | Type | Description | Example |
|---------------------|------|-------------|---------|
| `ATLAS_POSTGRESQL_CONNECTION_STRING` | string | PostgreSQL connection string (sensitive) | `postgresql://user:pass@localhost/db` |
| `ATLAS_POSTGRESQL_MAX_CONNECTIONS` | integer | Maximum connections (1-100) | `20` |
| `ATLAS_POSTGRESQL_CONNECTION_TIMEOUT_SECONDS` | integer | Connection timeout in seconds | `60` |
| `ATLAS_POSTGRESQL_STATEMENT_TIMEOUT_SECONDS` | integer | Statement timeout in seconds | `120` |
| `ATLAS_POSTGRESQL_SSL_MODE` | string | SSL mode: `disable`, `allow`, `prefer`, `require`, `verify-ca`, `verify-full` | `require` |

#### State Management

| Environment Variable | Type | Description | Example |
|---------------------|------|-------------|---------|
| `ATLAS_STATE_ENABLE_CHECKPOINTING` | boolean | Enable checkpointing | `true` |
| `ATLAS_STATE_CHECKPOINT_INTERVAL_SECONDS` | integer | Checkpoint interval in seconds | `60` |

#### Verification

| Environment Variable | Type | Description | Example |
|---------------------|------|-------------|---------|
| `ATLAS_VERIFICATION_ENABLE_VERIFICATION` | boolean | Enable data verification | `true` |

#### Logging

| Environment Variable | Type | Description | Example |
|---------------------|------|-------------|---------|
| `ATLAS_LOGGING_LOCAL_ENABLED` | boolean | Enable local file logging | `true` |
| `ATLAS_LOGGING_LOCAL_PATH` | string | Local log file path | `/var/log/atlas` |
| `ATLAS_LOGGING_LOCAL_ROTATION` | string | Log rotation: `daily` or `size` | `daily` |
| `ATLAS_LOGGING_LOCAL_MAX_SIZE_MB` | integer | Maximum log file size in MB | `200` |
| `ATLAS_LOGGING_AZURE_ENABLED` | boolean | Enable Azure Log Analytics | `false` |
| `ATLAS_LOGGING_AZURE_TENANT_ID` | string | Azure AD tenant ID | `00000000-0000-0000-0000-000000000000` |
| `ATLAS_LOGGING_AZURE_CLIENT_ID` | string | Azure AD client ID | `00000000-0000-0000-0000-000000000000` |
| `ATLAS_LOGGING_AZURE_CLIENT_SECRET` | string | Azure AD client secret (sensitive) | `secret` |
| `ATLAS_LOGGING_AZURE_LOG_ANALYTICS_WORKSPACE_ID` | string | Log Analytics workspace ID | `00000000-0000-0000-0000-000000000000` |
| `ATLAS_LOGGING_AZURE_DCR_IMMUTABLE_ID` | string | Data Collection Rule immutable ID | `dcr-00000000000000000000000000000000` |
| `ATLAS_LOGGING_AZURE_DCE_ENDPOINT` | string | Data Collection Endpoint URL | `https://my-dce.eastus-1.ingest.monitor.azure.com` |
| `ATLAS_LOGGING_AZURE_STREAM_NAME` | string | Azure stream name | `Custom-AtlasExport_CL` |

### Example: Containerized Deployment

```bash
# Docker run with environment variables
docker run -d \
  -e ATLAS_DATABASE_TARGET=postgresql \
  -e ATLAS_APPLICATION_LOG_LEVEL=info \
  -e ATLAS_OPENEHR_BASE_URL=https://prod-ehrbase.com \
  -e ATLAS_OPENEHR_USERNAME=atlas_prod \
  -e ATLAS_OPENEHR_PASSWORD="${OPENEHR_PASSWORD}" \
  -e ATLAS_OPENEHR_QUERY_TEMPLATE_IDS='["IDCR - Vital Signs.v1"]' \
  -e ATLAS_OPENEHR_QUERY_BATCH_SIZE=2000 \
  -e ATLAS_EXPORT_MODE=incremental \
  -e ATLAS_POSTGRESQL_CONNECTION_STRING="${PG_CONNECTION_STRING}" \
  -e ATLAS_POSTGRESQL_MAX_CONNECTIONS=20 \
  -e ATLAS_POSTGRESQL_SSL_MODE=require \
  -e ATLAS_LOGGING_LOCAL_ENABLED=true \
  -e ATLAS_LOGGING_LOCAL_PATH=/var/log/atlas \
  atlas:latest
```

### Example: Kubernetes ConfigMap and Secret

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: atlas-config
data:
  ATLAS_DATABASE_TARGET: "postgresql"
  ATLAS_APPLICATION_LOG_LEVEL: "info"
  ATLAS_OPENEHR_BASE_URL: "https://prod-ehrbase.com"
  ATLAS_OPENEHR_USERNAME: "atlas_prod"
  ATLAS_OPENEHR_QUERY_TEMPLATE_IDS: '["IDCR - Vital Signs.v1","IDCR - Lab Report.v1"]'
  ATLAS_OPENEHR_QUERY_BATCH_SIZE: "2000"
  ATLAS_EXPORT_MODE: "incremental"
  ATLAS_POSTGRESQL_MAX_CONNECTIONS: "20"
  ATLAS_POSTGRESQL_SSL_MODE: "require"
---
apiVersion: v1
kind: Secret
metadata:
  name: atlas-secrets
type: Opaque
stringData:
  ATLAS_OPENEHR_PASSWORD: "secret-password"
  ATLAS_POSTGRESQL_CONNECTION_STRING: "postgresql://user:pass@postgres:5432/openehr"
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: atlas
spec:
  template:
    spec:
      containers:
      - name: atlas
        image: atlas:latest
        envFrom:
        - configMapRef:
            name: atlas-config
        - secretRef:
            name: atlas-secrets
```

## Complete Example

See [`atlas.toml.example`](../atlas.toml.example) for a complete, commented configuration file.

## Common Configurations

### Clinical Research Export

Export all compositions for specific templates to support research analytics:

```toml
[export]
mode = "full"
export_composition_format = "preserve"

[openehr.query]
template_ids = ["IDCR - Adverse Reaction List.v1", "IDCR - Problem List.v1", "IDCR - Vital Signs.v1"]
ehr_ids = []  # All patients
batch_size = 1000
```

### Incremental Daily Sync

Nightly incremental sync for operational dashboards:

```toml
[export]
mode = "incremental"
export_composition_format = "flatten"

[openehr.query]
template_ids = ["IDCR - Vital Signs.v1"]
batch_size = 2000
parallel_ehrs = 16

[state]
enable_checkpointing = true
checkpoint_interval_seconds = 60
```

### ML Feature Extraction

High-throughput export for machine learning pipelines:

```toml
[export]
mode = "full"
export_composition_format = "flatten"

[openehr.query]
template_ids = ["IDCR - Lab Results.v1"]
batch_size = 5000
parallel_ehrs = 32

[cosmosdb]
max_concurrency = 50

[verification]
enable_verification = true
```

### Development/Testing

Safe configuration for development and testing:

```toml
[application]
log_level = "debug"

[export]
dry_run = true  # Don't write to database

[openehr.query]
template_ids = ["Test Template.v1"]
ehr_ids = ["test-ehr-001"]  # Single test patient
batch_size = 100

[logging]
local_enabled = true
local_path = "./logs"
```

## Validation

Always validate your configuration before running exports:

```bash
atlas validate-config -c atlas.toml
```

This checks:

- Required fields are present
- Values are within valid ranges
- Authentication credentials are provided when needed
- URLs have correct format
- Template IDs are not empty

## Security Best Practices

### Credential Protection

Atlas implements secure credential handling to protect sensitive information:

- **Memory Protection**: All credentials (passwords, keys, secrets) are automatically zeroized in memory when no longer needed
- **No Logging**: Credentials are never written to log files or exposed in debug output
- **Redacted Debug Output**: Debug representations show `Secret([REDACTED])` instead of actual values
- **Explicit Access**: Code must explicitly call `expose_secret()` to access credential values, making security audits easier

**Protected Credentials:**
- OpenEHR password (`openehr.password`)
- Cosmos DB key (`cosmosdb.key`)
- PostgreSQL connection string (`postgresql.connection_string`)
- Azure client secret (`logging.azure_client_secret`)

### Configuration Security

1. **Never commit credentials to version control**
   - Use environment variables for sensitive values (recommended)
   - Add `atlas.toml` to `.gitignore`
   - Use `.env` file for local development (also add to `.gitignore`)

2. **Use strong access controls**
   - Limit OpenEHR user permissions to read-only
   - Use Cosmos DB keys with minimum required permissions
   - Rotate keys regularly
   - Use Azure Managed Identity when possible

3. **Enable TLS verification** (Enforced in Production)
   - **REQUIRED**: Set `environment = "production"` for production deployments
   - TLS verification is automatically enforced in production environments
   - Configuration validation will fail if `tls_verify = false` in production
   - Only disable for local development/testing with `environment = "development"`
   - Use `tls_ca_cert` for custom CA certificates in production
   - A security warning is logged whenever TLS verification is disabled

4. **Secure log files**
   - Ensure log directory has appropriate permissions
   - Logs may contain PHI/PII - treat as sensitive data
   - Configure log rotation to prevent disk space issues
   - Credentials are automatically excluded from logs

## Troubleshooting

### Configuration Not Found

```bash
Error: Configuration file not found: atlas.toml
```

**Solution**: Create a configuration file using `atlas init` or specify the path with `-c`:

```bash
atlas export -c /path/to/atlas.toml
```

### Invalid Configuration

```bash
Error: Invalid export.mode 'invalid'. Must be one of: full, incremental
```

**Solution**: Check the error message for the specific validation failure and correct the value.

### Environment Variable Not Substituted

```bash
Error: cosmosdb.key cannot be empty
```

**Solution**: Ensure the environment variable is set before running Atlas:

```bash
export ATLAS_OPENEHR_USERNAME="your-username"
export ATLAS_OPENEHR_PASSWORD="your-password"
export ATLAS_COSMOSDB_KEY="your-cosmos-key"
atlas export
```

### Connection Failures

```bash
Error: Failed to connect to OpenEHR server
```

**Solution**:

- Verify `base_url` is correct and accessible
- Check firewall rules and network connectivity
- Verify credentials are correct
- Check TLS certificate if using `tls_verify = true`

### TLS Certificate Verification Errors

```bash
Error: A certificate chain processed, but terminated in a root certificate which is not trusted by the trust provider
```

**Cause**: The OpenEHR server is using a self-signed certificate or a certificate from an untrusted CA.

**Solutions**:

1. **For Development/Testing ONLY** - Disable certificate verification:

   ```toml
   # Set environment to development
   environment = "development"

   [openehr]
   tls_verify = false
   # OR
   tls_verify_certificates = false
   ```

   **⚠️ WARNING**: This will log a security warning at startup. This configuration is **BLOCKED** in production environments.

2. **For Production (Recommended)** - Use a custom CA certificate:

   ```toml
   # Set environment to production
   environment = "production"

   [openehr]
   tls_verify = true  # Required in production
   tls_ca_cert = "/path/to/ca-certificate.pem"
   ```

3. **Best Practice** - Use a certificate from a trusted CA (Let's Encrypt, DigiCert, etc.)

   ```toml
   environment = "production"

   [openehr]
   tls_verify = true  # Works automatically with trusted CAs
   ```

**🔒 CRITICAL SECURITY NOTE**:
- **NEVER disable TLS verification in production environments**
- Atlas enforces this by blocking `tls_verify = false` when `environment = "production"`
- Disabling TLS verification exposes your application to man-in-the-middle attacks
- This violates healthcare data protection regulations (HIPAA, GDPR, etc.)

---

For more information, see:

- [User Guide](user-guide.md) - Step-by-step usage instructions
- [Architecture Documentation](architecture.md) - System design and components
- [Developer Guide](developer-guide.md) - Contributing and development setup
