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

Example:

```toml
username = "${ATLAS_OPENEHR_USERNAME}"
password = "${ATLAS_OPENEHR_PASSWORD}"
key = "${ATLAS_COSMOSDB_KEY}"
```

## Configuration Sections

### Application

Application-level settings that control Atlas behavior.

```toml
[application]
name = "atlas"
version = "1.0.0"
log_level = "info"
dry_run = false
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `name` | string | "atlas" | Application name for logging and identification |
| `version` | string | "1.0.0" | Application version |
| `log_level` | string | "info" | Log verbosity: `trace`, `debug`, `info`, `warn`, `error` |
| `dry_run` | boolean | false | If true, simulate export without writing to Cosmos DB |

### OpenEHR

Configuration for connecting to the OpenEHR server (EHRBase).

```toml
[openehr]
base_url = "https://ehrbase.example.com/ehrbase"
vendor = "ehrbase"
auth_type = "basic"
username = "${ATLAS_OPENEHR_USERNAME}"
password = "${ATLAS_OPENEHR_PASSWORD}"
tls_verify = true
tls_verify_certificates = true
timeout_seconds = 60
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `base_url` | string | **required** | Base URL of the OpenEHR server (e.g., `https://ehrbase.localhost/ehrbase`). Do not include `/rest/openehr/v1` - Atlas adds this automatically. |
| `vendor` | string | "ehrbase" | Vendor implementation (currently only "ehrbase" supported) |
| `auth_type` | string | "basic" | Authentication type: `basic` or `openid` (openid not yet implemented) |
| `username` | string | null | Username for basic authentication (required if auth_type is "basic") |
| `password` | string | null | Password for basic authentication (required if auth_type is "basic") |
| `tls_verify` | boolean | true | Enable TLS certificate verification (alias for `tls_verify_certificates`) |
| `tls_verify_certificates` | boolean | true | Enable TLS certificate verification (alias for `tls_verify`) |
| `tls_ca_cert` | string | null | Optional path to custom CA certificate file |
| `timeout_seconds` | integer | 60 | Request timeout in seconds |

**Note on TLS Verification:**

- Both `tls_verify` and `tls_verify_certificates` control the same setting - use either one
- Set to `false` only for development/testing with self-signed certificates
- For production with self-signed certificates, use `tls_ca_cert` to specify your CA certificate instead
- **Security Warning**: Disabling TLS verification (`tls_verify = false`) should only be used in development environments

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
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `mode` | string | "incremental" | Export mode: `full` (all data) or `incremental` (only new/changed data since last export) |
| `export_composition_format` | string | "preserve" | Data format: `preserve` (exact FLAT JSON structure) or `flatten` (convert paths to field names) |
| `database_target` | string | **required** | Database backend: `cosmosdb` or `postgresql` |
| `max_retries` | integer | 3 | Maximum retry attempts for failed exports (0-10) |
| `retry_backoff_ms` | array[integer] | [1000, 2000, 4000] | Retry delay intervals in milliseconds |

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

Optional data integrity verification using checksums.

```toml
[verification]
enable_verification = false
checksum_algorithm = "sha256"
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enable_verification` | boolean | false | Enable post-export data verification |
| `checksum_algorithm` | string | "sha256" | Checksum algorithm: `sha256` or `sha512` |

**Verification Process:**

- When enabled, Atlas calculates checksums of exported data
- Checksums are stored in the `atlas_metadata.checksum` field
- Post-export verification compares stored checksums with recalculated values
- Verification adds overhead; recommended only for critical data exports

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
dry_run = true  # Don't write to Cosmos DB

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

1. **Never commit credentials to version control**
   - Use environment variables for sensitive values
   - Add `atlas.toml` to `.gitignore`

2. **Use strong access controls**
   - Limit OpenEHR user permissions to read-only
   - Use Cosmos DB keys with minimum required permissions
   - Rotate keys regularly

3. **Enable TLS verification**
   - Keep `tls_verify = true` in production
   - Only disable for local development with self-signed certificates

4. **Secure log files**
   - Ensure log directory has appropriate permissions
   - Logs may contain PHI/PII - treat as sensitive data
   - Configure log rotation to prevent disk space issues

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

1. **For Development/Testing** - Disable certificate verification:

   ```toml
   [openehr]
   tls_verify = false
   # OR
   tls_verify_certificates = false
   ```

2. **For Production** - Use a custom CA certificate:

   ```toml
   [openehr]
   tls_verify = true
   tls_ca_cert = "/path/to/ca-certificate.pem"
   ```

3. **Best Practice** - Use a certificate from a trusted CA (Let's Encrypt, DigiCert, etc.)

**Security Note**: Never disable TLS verification in production environments. This makes your connection vulnerable to man-in-the-middle attacks.

---

For more information, see:

- [User Guide](user-guide.md) - Step-by-step usage instructions
- [Architecture Documentation](architecture.md) - System design and components
- [Developer Guide](developer-guide.md) - Contributing and development setup
