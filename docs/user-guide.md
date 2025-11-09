# Atlas User Guide

This guide provides step-by-step instructions for setting up and using Atlas to export OpenEHR data to Azure Cosmos DB.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Common Use Cases](#common-use-cases)
- [Command Reference](#command-reference)
- [Troubleshooting](#troubleshooting)
- [FAQ](#faq)
- [Best Practices](#best-practices)

## Prerequisites

Before using Atlas, ensure you have:

### Required

1. **Rust 1.70 or later** (for building from source)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustc --version  # Should be 1.70+
   ```

2. **Access to an OpenEHR server**
   - EHRBase 0.30 or later
   - REST API v1.1.x endpoint
   - Valid credentials (username/password for Basic Auth)
   - Network connectivity to the server

3. **Azure Cosmos DB account**
   - Cosmos DB Core (SQL) API account
   - Database created (e.g., `openehr_data`)
   - Primary or secondary access key
   - Sufficient RU/s provisioned (recommend starting with 1000 RU/s)

### Optional

- **Docker** (for containerized deployment)
- **Kubernetes/AKS** (for production deployments)
- **Azure Application Insights** (for centralized logging)

## Installation

### Option 1: Build from Source (Recommended)

```bash
# Clone the repository
git clone https://github.com/erikhoward/atlas.git
cd atlas

# Build the release binary
cargo build --release

# The binary will be at target/release/atlas
# Optionally, install it to your PATH
sudo cp target/release/atlas /usr/local/bin/
```

### Option 2: Pre-built Binaries

Download the latest release from the [GitHub Releases page](https://github.com/erikhoward/atlas/releases):

```bash
# Linux
wget https://github.com/erikhoward/atlas/releases/download/v2.0.0/atlas-linux-x86_64.tar.gz
tar -xzf atlas-linux-x86_64.tar.gz
sudo mv atlas /usr/local/bin/

# macOS
wget https://github.com/erikhoward/atlas/releases/download/v2.0.0/atlas-macos-x86_64.tar.gz
tar -xzf atlas-macos-x86_64.tar.gz
sudo mv atlas /usr/local/bin/
```

### Option 3: Docker

```bash
# Pull the Docker image
docker pull erikhoward/atlas:latest

# Run Atlas in a container
docker run --rm -v $(pwd)/atlas.toml:/app/atlas.toml erikhoward/atlas:latest export
```

### Verify Installation

```bash
atlas --version
# Output: atlas 2.0.0

atlas --help
# Shows available commands
```

## Quick Start

### Step 1: Generate Configuration

Create a sample configuration file:

```bash
# Generate minimal configuration
atlas init

# Or generate with examples and comments
atlas init --with-examples --output atlas.toml
```

This creates `atlas.toml` in the current directory.

### Step 2: Configure Atlas

Edit `atlas.toml` with your environment details:

```toml
[application]
name = "atlas"
version = "1.0.0"
log_level = "info"

[openehr]
base_url = "https://your-ehrbase-server.com/ehrbase/rest/openehr/v1"
username = "${ATLAS_OPENEHR_USERNAME}"  # Use environment variable
password = "${ATLAS_OPENEHR_PASSWORD}"  # Use environment variable

[openehr.query]
template_ids = ["Your Template ID.v1"]
batch_size = 1000

[cosmosdb]
endpoint = "https://your-account.documents.azure.com:443/"
key = "${ATLAS_COSMOSDB_KEY}"  # Use environment variable
database_name = "openehr_data"

[export]
mode = "incremental"
export_composition_format = "preserve"
```

### Step 3: Set Environment Variables

Set sensitive credentials as environment variables:

```bash
export ATLAS_OPENEHR_USERNAME="your-openehr-username"
export ATLAS_OPENEHR_PASSWORD="your-openehr-password"
export ATLAS_COSMOSDB_KEY="your-cosmos-db-key"
```

For persistent configuration, add to `~/.bashrc` or `~/.zshrc`:

```bash
echo 'export ATLAS_OPENEHR_USERNAME="your-username"' >> ~/.bashrc
echo 'export ATLAS_OPENEHR_PASSWORD="your-password"' >> ~/.bashrc
echo 'export ATLAS_COSMOSDB_KEY="your-key"' >> ~/.bashrc
source ~/.bashrc
```

### Step 4: Validate Configuration

Before running an export, validate your configuration:

```bash
atlas validate-config -c atlas.toml
```

Expected output:
```
✓ Configuration is valid
✓ OpenEHR connection successful
✓ Cosmos DB connection successful
✓ All template IDs are valid
```

### Step 5: Run Your First Export

Start with a dry run to preview the export:

```bash
atlas export --dry-run -c atlas.toml
```

If everything looks good, run the actual export:

```bash
atlas export -c atlas.toml
```

**Note**: If using the default config file name (`atlas.toml`), you can omit the `-c` flag:

```bash
# These are equivalent when using atlas.toml
atlas export
atlas export -c atlas.toml
```

**When building from source with cargo**:

```bash
# The --config flag must come BEFORE the subcommand
cargo run -- --config atlas.toml export

# Or use the short form
cargo run -- -c atlas.toml export

# Or omit for default config file
cargo run -- export
```

### Step 6: Understanding Export Output

After the export completes, you'll see a summary like this:

```
Export Summary:
  Total Compositions: 1,234
  Successful Exports: 1,200
  Failed Exports: 4
  Duplicates Skipped: 30
  Success Rate: 97.1%
  Duration: 2m 15s
```

**What each metric means**:
- **Total Compositions**: Total number of compositions processed (successful + failed)
- **Successful Exports**: Compositions successfully written to Cosmos DB
- **Failed Exports**: Compositions that failed during transformation or insertion
- **Duplicates Skipped**: Compositions already in Cosmos DB (skipped to avoid duplicates)
- **Success Rate**: Percentage of successful exports out of total processed
- **Duration**: Total time taken for the export

### Step 7: Check Export Status

View the current export status and watermarks:

```bash
atlas status -c atlas.toml
```

## Common Use Cases

### Use Case 1: Initial Full Export

Export all compositions for specific templates to populate Cosmos DB:

**Configuration** (`atlas.toml`):
```toml
[export]
mode = "full"  # Export all data
export_composition_format = "preserve"

[openehr.query]
template_ids = [
    "IDCR - Adverse Reaction List.v1",
    "IDCR - Problem List.v1",
    "IDCR - Vital Signs.v1"
]
ehr_ids = []  # All patients
batch_size = 1000
parallel_ehrs = 8
```

**Commands**:
```bash
# Validate first
atlas validate-config

# Run export
atlas export

# Monitor progress in logs
tail -f /var/log/atlas/atlas.log
```

### Use Case 2: Incremental Daily Sync

Set up a nightly cron job for incremental exports:

**Configuration** (`atlas.toml`):
```toml
[export]
mode = "incremental"  # Only new/changed data
export_composition_format = "flatten"

[openehr.query]
template_ids = ["IDCR - Vital Signs.v1"]
batch_size = 2000
parallel_ehrs = 16

[state]
enable_checkpointing = true
checkpoint_interval_seconds = 60
```

**Cron Job** (`/etc/cron.d/atlas-sync`):
```bash
# Run daily at 2 AM
0 2 * * * atlas /usr/local/bin/atlas export -c /etc/atlas/atlas.toml >> /var/log/atlas/cron.log 2>&1
```

### Use Case 3: Export Specific Patients

Export data for a specific set of patients (e.g., for research cohort):

**Configuration**:
```toml
[openehr.query]
template_ids = ["IDCR - Lab Results.v1"]
ehr_ids = [
    "ehr-001",
    "ehr-002",
    "ehr-003"
]
```

**Command**:
```bash
atlas export
```

### Use Case 4: Time-Range Export

Export compositions within a specific time range:

**Configuration**:
```toml
[openehr.query]
template_ids = ["IDCR - Vital Signs.v1"]
time_range_start = "2024-01-01T00:00:00Z"
time_range_end = "2024-12-31T23:59:59Z"
```

### Use Case 5: High-Throughput ML Pipeline

Optimize for maximum throughput when exporting large datasets:

**Configuration**:
```toml
[export]
mode = "full"
export_composition_format = "flatten"

[openehr.query]
batch_size = 5000  # Maximum batch size
parallel_ehrs = 32  # High parallelism

[cosmosdb]
max_concurrency = 50  # High Cosmos DB concurrency

[verification]
enable_verification = false  # Disable for speed
```

**Command**:
```bash
# Run with increased log level for monitoring
atlas export --log-level debug
```

### Use Case 6: Development and Testing

Safe configuration for testing without affecting production data:

**Configuration** (`atlas-dev.toml`):
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
local_path = "./logs"  # Local directory
```

**Command**:
```bash
atlas export -c atlas-dev.toml
```

## Dry-Run Mode

Dry-run mode allows you to test your export configuration and preview what would be exported **without writing any data to the database**. This is essential for:

- **Testing configurations** before running production exports
- **Validating connectivity** to OpenEHR and database
- **Previewing export scope** (which compositions would be exported)
- **Debugging issues** without affecting production data
- **Training and demonstrations** without data modification

### How Dry-Run Works

When dry-run mode is enabled:

1. ✅ **Reads data** from OpenEHR normally
2. ✅ **Transforms compositions** according to your configuration
3. ✅ **Validates** all data processing logic
4. ✅ **Logs** what would be written
5. ❌ **Skips all database writes** (compositions and watermarks)
6. ✅ **Generates summary** showing what would have been exported

### Enabling Dry-Run Mode

There are two ways to enable dry-run mode:

#### Option 1: CLI Flag (Recommended)

Use the `--dry-run` flag when running the export command:

```bash
atlas export --dry-run
```

This is the recommended approach as it's explicit and doesn't require config changes.

#### Option 2: Configuration File

Set `dry_run = true` in your configuration file:

```toml
[export]
mode = "incremental"
export_composition_format = "preserve"
dry_run = true  # Enable dry-run mode
```

Then run normally:

```bash
atlas export
```

### Dry-Run Output

When dry-run mode is active, you'll see:

```
🔍 DRY RUN MODE - No data will be written to the database

Starting export process...
DRY RUN: Would insert 150 compositions (preserved format)
DRY RUN: Would save watermark

Export Summary:
  Total EHRs: 5
  Total Compositions: 150
  Successful: 150
  Failed: 0
  Duration: 12.5s
  🔍 DRY RUN MODE - No data was written to the database
```

### Best Practices

1. **Always test first**: Run with `--dry-run` before any production export
2. **Validate configuration**: Use dry-run to catch config errors early
3. **Check scope**: Verify the number of compositions matches expectations
4. **Test incremental mode**: Ensure watermarks would be updated correctly
5. **Performance testing**: Dry-run can help estimate export duration

### Example Workflow

```bash
# 1. Validate configuration
atlas validate-config

# 2. Test with dry-run
atlas export --dry-run

# 3. Review the output and logs
# Check: Number of compositions, any errors, duration

# 4. If everything looks good, run actual export
atlas export
```

### Limitations

Dry-run mode does NOT:

- ❌ Test database write permissions (since no writes occur)
- ❌ Test database capacity or throughput limits
- ❌ Verify database schema compatibility
- ❌ Test retry logic for database failures

For these scenarios, consider using a test database environment.

## Command Reference

### `atlas export`

Execute data export from OpenEHR to Cosmos DB.

**Usage**:
```bash
atlas export [OPTIONS]
```

**Options**:
- `-c, --config <FILE>`: Configuration file path (default: `atlas.toml`)
- `-y, --yes`: Skip confirmation prompt
- `--dry-run`: Simulate export without writing to database
- `--template-id <ID>`: Override template IDs from config (can be specified multiple times)
- `--ehr-id <ID>`: Override EHR IDs from config (can be specified multiple times)
- `--mode <MODE>`: Override export mode (`full` or `incremental`)
- `-l, --log-level <LEVEL>`: Override log level (`trace`, `debug`, `info`, `warn`, `error`)

**Examples**:
```bash
# Basic export
atlas export

# Dry run
atlas export --dry-run

# Override template IDs
atlas export --template-id "Template1.v1" --template-id "Template2.v1"

# Skip confirmation
atlas export -y

# Custom config file
atlas export -c /etc/atlas/production.toml
```

**Exit Codes**:
- `0`: Success (all compositions exported)
- `1`: Partial success (some compositions failed)
- `2`: Configuration error
- `3`: Authentication error
- `4`: Connection error
- `5`: Fatal error
- `130`: Interrupted by SIGINT (Ctrl+C)
- `143`: Interrupted by SIGTERM (graceful termination signal)

**Graceful Shutdown**:

Atlas supports graceful shutdown for long-running exports. When you press Ctrl+C or send a SIGTERM signal:

1. **Current batch completes**: Atlas finishes processing the current batch to avoid partial data
2. **Watermark saved**: Progress is saved to the database with `Interrupted` status
3. **Clean exit**: Atlas exits with code 130 (SIGINT) or 143 (SIGTERM)
4. **Resume support**: Re-run the same command to continue from the checkpoint

```bash
# Start an export
atlas export -c atlas.toml

# Press Ctrl+C to gracefully stop
# Output:
# ⚠️  Shutdown signal received, completing current batch...
# ⚠️  Export interrupted gracefully. Progress saved.
#    Run the same command to resume from checkpoint.

# Resume from where it left off
atlas export -c atlas.toml
```

**Configuration**:

The shutdown timeout can be configured in your `atlas.toml`:

```toml
[export]
# Graceful shutdown timeout in seconds (default: 30)
# This is the maximum time to wait for the current batch to complete
# Should align with container orchestration grace periods
shutdown_timeout_secs = 30
```

**Best Practices**:
- Set `shutdown_timeout_secs` to match your container orchestration grace period (e.g., Kubernetes default is 30s)
- For very large batches, consider increasing the timeout or reducing batch size
- Monitor logs to ensure batches complete within the timeout window
- In Docker/Kubernetes, use `docker stop` or `kubectl delete pod` for graceful shutdown (not `docker kill` or `kubectl delete pod --force`)

### `atlas validate-config`

Validate configuration file and test connections.

**Usage**:
```bash
atlas validate-config [OPTIONS]
```

**Options**:
- `-c, --config <FILE>`: Configuration file path (default: `atlas.toml`)

**Example**:
```bash
atlas validate-config -c atlas.toml
```

### `atlas status`

Display export status and watermarks.

**Usage**:
```bash
atlas status [OPTIONS]
```

**Options**:
- `-c, --config <FILE>`: Configuration file path (default: `atlas.toml`)
- `--template-id <ID>`: Filter by template ID
- `--ehr-id <ID>`: Filter by EHR ID

**Examples**:
```bash
# Show all watermarks
atlas status

# Filter by template
atlas status --template-id "IDCR - Vital Signs.v1"

# Filter by EHR
atlas status --ehr-id "ehr-001"
```

### `atlas init`

Generate sample configuration file.

**Usage**:
```bash
atlas init [OPTIONS]
```

**Options**:
- `-o, --output <FILE>`: Output file path (default: `atlas.toml`)
- `--with-examples`: Include detailed examples and comments
- `--force`: Overwrite existing file

**Examples**:
```bash
# Generate minimal config
atlas init

# Generate with examples
atlas init --with-examples

# Custom output path
atlas init -o /etc/atlas/atlas.toml

# Overwrite existing
atlas init --force
```

## Troubleshooting

### Connection Issues

#### Problem: Cannot connect to OpenEHR server

**Error**:
```
Error: Failed to connect to OpenEHR server: Connection refused
```

**Solutions**:
1. Verify the `base_url` is correct and accessible
2. Check firewall rules and network connectivity
3. Test with `curl`:
   ```bash
   curl -u username:password https://your-ehrbase-server.com/ehrbase/rest/openehr/v1/ehr
   ```
4. Verify TLS certificate if using `tls_verify = true`

#### Problem: TLS certificate verification failure

**Error**:
```
Error: A certificate chain processed, but terminated in a root certificate which is not trusted by the trust provider
```

**Cause**: The OpenEHR server is using a self-signed certificate or a certificate from an untrusted CA (common with Traefik, nginx reverse proxies in development).

**Solutions**:

1. **Quick fix for development/testing** - Disable certificate verification in `atlas.toml`:
   ```toml
   [openehr]
   base_url = "https://ehrbase.localhost/ehrbase"
   username = "${ATLAS_OPENEHR_USERNAME}"
   password = "${ATLAS_OPENEHR_PASSWORD}"
   tls_verify = false  # Disable TLS verification
   ```

2. **Production solution** - Provide a custom CA certificate:
   ```toml
   [openehr]
   tls_verify = true
   tls_ca_cert = "/path/to/your-ca-certificate.pem"
   ```

3. **Best practice** - Use a certificate from a trusted CA (Let's Encrypt, DigiCert, etc.)

**Security Warning**: Never disable TLS verification (`tls_verify = false`) in production environments. This makes your connection vulnerable to man-in-the-middle attacks. Only use this for local development with self-signed certificates.

#### Problem: Cosmos DB authentication failure

**Error**:
```
Error: Cosmos DB authentication failed: Unauthorized
```

**Solutions**:
1. Verify the `endpoint` URL is correct
2. Check that the `key` is valid (primary or secondary key)
3. Ensure the environment variable is set:
   ```bash
   echo $ATLAS_COSMOS_KEY
   ```
4. Test connection with Azure CLI:
   ```bash
   az cosmosdb show --name your-account --resource-group your-rg
   ```

### Configuration Issues

#### Problem: Template IDs not found

**Error**:
```
Error: Template 'Unknown Template.v1' not found in OpenEHR server
```

**Solutions**:
1. Verify template IDs exist in the OpenEHR server
2. Check for typos in template names
3. List available templates:
   ```bash
   curl -u username:password https://your-ehrbase-server.com/ehrbase/rest/openehr/v1/definition/template/adl1.4
   ```

#### Problem: Invalid batch size

**Error**:
```
Error: openehr.query.batch_size must be between 100 and 5000, got 50
```

**Solution**: Adjust `batch_size` in configuration to be within the valid range (100-5000).

### Performance Issues

#### Problem: Export is very slow

**Symptoms**: Export takes hours for moderate datasets

**Solutions**:
1. Increase `parallel_ehrs` (default: 8, try 16-32)
2. Increase `batch_size` (default: 1000, try 2000-5000)
3. Increase `max_concurrency` for Cosmos DB (default: 10, try 20-50)
4. Check Cosmos DB RU consumption and scale up if throttled
5. Monitor network latency between Atlas and servers

#### Problem: High memory usage

**Symptoms**: Atlas consumes excessive memory

**Solutions**:
1. Decrease `batch_size` (try 500-1000)
2. Decrease `parallel_ehrs` (try 4-8)
3. Monitor with:
   ```bash
   ps aux | grep atlas
   ```

### Data Issues

#### Problem: Duplicate compositions

**Error**:
```
Warning: Skipped 10 duplicate compositions
```

**Explanation**: This is normal behavior. Atlas detects and skips compositions that already exist in Cosmos DB.

**Action**: No action needed unless the count is unexpectedly high.

#### Problem: Checksum verification failures

**Error**:
```
Error: Verification failed: 5 compositions have checksum mismatches
```

**Solutions**:
1. Check for data corruption during transfer
2. Verify network stability
3. Re-export failed compositions:
   ```bash
   atlas export --mode full --template-id "Affected Template.v1"
   ```

## FAQ

### Q: How do I resume a failed export?

**A**: Atlas automatically resumes from the last checkpoint when running in incremental mode. Simply run the export command again:
```bash
atlas export
```

The watermarks stored in the control container track the last successfully exported composition, so Atlas will continue from where it left off.

### Q: Can I export to multiple Cosmos DB accounts?

**A**: Not directly in a single run. You can:
1. Run Atlas multiple times with different configuration files
2. Use a script to orchestrate multiple exports
3. Replicate data between Cosmos DB accounts using Azure Data Factory

### Q: How do I delete exported data?

**A**: Atlas does not provide a delete command. To remove data:
1. Use Azure Portal to delete containers or documents
2. Use Azure CLI:
   ```bash
   az cosmosdb sql container delete --account-name your-account --database-name openehr_data --name compositions_template_id
   ```
3. Use Cosmos DB Data Explorer

### Q: What happens if the OpenEHR server is unavailable during export?

**A**: Atlas will retry failed requests with exponential backoff (configurable). If all retries fail, the export will stop and log the error. You can resume the export once the server is available.

### Q: How do I monitor export progress?

**A**: Atlas provides several ways to monitor progress:
1. **Console output**: Real-time progress during export
2. **Log files**: Detailed logs in `/var/log/atlas/` (or configured path)
3. **Azure Application Insights**: If enabled, view metrics and logs in Azure Portal
4. **Status command**: Check watermarks and last export status

### Q: Can I run multiple Atlas instances concurrently?

**A**: Yes, but with caution:
- Different template IDs: Safe to run concurrently
- Same template ID, different EHR IDs: Safe to run concurrently
- Same template ID and EHR IDs: Not recommended (may cause duplicate processing)

### Q: How do I upgrade Atlas?

**A**:
1. Backup your configuration file
2. Download/build the new version
3. Replace the binary
4. Review release notes for breaking changes
5. Test with `--dry-run` before production use

## Best Practices

### 1. Configuration Management

- **Version control**: Store configuration files in Git (exclude sensitive values)
- **Environment variables**: Use for all sensitive credentials
- **Separate configs**: Maintain separate files for dev, staging, and production
- **Validation**: Always run `validate-config` before production exports

### 2. Scheduling and Automation

- **Cron jobs**: Use for regular incremental exports
- **Monitoring**: Set up alerts for failed exports
- **Logging**: Rotate logs to prevent disk space issues
- **Notifications**: Send email/Slack notifications on failures

Example cron with error notification:
```bash
0 2 * * * /usr/local/bin/atlas export -c /etc/atlas/atlas.toml || echo "Atlas export failed" | mail -s "Atlas Alert" admin@example.com
```

### 3. Performance Tuning

- **Start conservative**: Begin with default settings
- **Monitor metrics**: Track export duration, RU consumption, error rates
- **Incremental tuning**: Adjust one parameter at a time
- **Load testing**: Test with production-like data volumes before go-live

### 4. Security

- **Credential Protection**: Atlas automatically protects credentials in memory and never logs them
  - All passwords, keys, and secrets are zeroized when no longer needed
  - Debug output shows `Secret([REDACTED])` instead of actual values
  - Use environment variables for all sensitive values
- **Least privilege**: Use read-only OpenEHR credentials
- **Key rotation**: Rotate Cosmos DB keys regularly
- **TLS verification**: Keep `tls_verify = true` in production
- **Log sanitization**: PHI/PII and credentials are automatically excluded from logs

### 5. Data Quality

- **Verification**: Enable checksums for critical data exports
- **Reconciliation**: Periodically compare counts between OpenEHR and Cosmos DB
- **Testing**: Test with sample data before production exports
- **Backup**: Maintain backups of Cosmos DB data

### 6. Operational Excellence

- **Documentation**: Document your specific configuration and procedures
- **Runbooks**: Create runbooks for common issues
- **Change management**: Test configuration changes in non-production first
- **Capacity planning**: Monitor growth and plan for scaling

---

For more information, see:
- [Configuration Guide](configuration.md) - Detailed configuration reference
- [Architecture Documentation](architecture.md) - System design and components
- [Developer Guide](developer-guide.md) - Contributing and development setup

