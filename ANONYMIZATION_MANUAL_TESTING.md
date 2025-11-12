# Anonymization Feature - Manual Testing Guide

This guide provides comprehensive instructions for manually testing the anonymization feature in Atlas.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Configuration](#configuration)
3. [Test Scenarios](#test-scenarios)
4. [Verification Steps](#verification-steps)
5. [Performance Testing](#performance-testing)
6. [Troubleshooting](#troubleshooting)

---

## Prerequisites

### 1. Build the Project

```bash
cargo build --release
```

### 2. Verify All Tests Pass

```bash
# Run all tests
cargo test

# Expected results:
# - 207 library tests passing
# - 41 integration tests passing (9 + 17 + 15)
# - 56 doctests passing
```

### 3. Prepare Configuration File

Atlas uses a TOML configuration file for database and OpenEHR settings.

**If you already have `atlas.toml`**, simply add the `[anonymization]` section at the end (see below).

**If you need to create a new config**, use:
```bash
./target/release/atlas init --with-examples
```

**Minimal configuration example** (`atlas.toml`):

```toml
# Database target selection
database_target = "cosmosdb"  # or "postgresql"

# Runtime environment (affects TLS validation)
environment = "development"  # or "staging" or "production"

[application]
log_level = "info"
dry_run = false

[openehr]
base_url = "https://your-openehr-server.com/ehrbase/rest/openehr/v1"
vendor = "ehrbase"
auth_type = "basic"
username = "${ATLAS_OPENEHR_USERNAME}"
password = "${ATLAS_OPENEHR_PASSWORD}"
tls_verify = true

[openehr.query]
template_ids = ["HTN_Monitoring.v1"]
batch_size = 1000
parallel_ehrs = 8

[export]
mode = "incremental"  # or "full"
export_composition_format = "preserve"  # or "flatten"
max_retries = 3
retry_backoff_ms = [1000, 2000, 4000]

# Cosmos DB configuration (if using cosmosdb)
[cosmosdb]
endpoint = "https://your-account.documents.azure.com:443/"
key = "${ATLAS_COSMOSDB_KEY}"
database_name = "openehr_data"
control_container = "atlas_control"
data_container_prefix = "compositions"
partition_key = "/ehr_id"
max_concurrency = 10
request_timeout_seconds = 60

# PostgreSQL configuration (if using postgresql)
# [postgresql]
# connection_string = "postgresql://atlas_user:${ATLAS_PG_PASSWORD}@localhost:5432/openehr_data?sslmode=require"
# max_connections = 20
# connection_timeout_seconds = 30
# statement_timeout_seconds = 60
# ssl_mode = "require"

[state]
enable_checkpointing = true
checkpoint_interval_seconds = 30

[verification]
enable_verification = false
checksum_algorithm = "sha256"

[logging]
local_enabled = true
local_path = "/var/log/atlas"
local_rotation = "daily"
local_max_size_mb = 100
azure_enabled = false

# Anonymization configuration (add this section)
[anonymization]
enabled = true
mode = "hipaa_safe_harbor"  # or "gdpr"
strategy = "token"          # or "redact"
dry_run = true              # Start with dry-run for testing

[anonymization.audit]
enabled = true
log_path = "./audit/anonymization.log"
json_format = true
```

### 4. Add Anonymization Configuration

**To enable anonymization, add this section to your existing `atlas.toml`:**

```toml
# ============================================================================
# ANONYMIZATION (Phase 1)
# ============================================================================

[anonymization]
enabled = true
mode = "hipaa_safe_harbor"  # hipaa_safe_harbor | gdpr
strategy = "token"          # token | redact
dry_run = true              # Start with dry-run for testing

[anonymization.audit]
enabled = true
log_path = "./audit/anonymization.log"
json_format = true
```

**Configuration Options:**

- `enabled`: Set to `true` to enable anonymization
- `mode`:
  - `"hipaa_safe_harbor"` - Detects 18 HIPAA Safe Harbor identifiers
  - `"gdpr"` - Detects HIPAA identifiers + GDPR quasi-identifiers
- `strategy`:
  - `"token"` - Replace PII with random tokens (e.g., `TOKEN_EMAIL_a1b2c3d4`)
  - `"redact"` - Replace PII with `[REDACTED_*]` markers
- `dry_run`:
  - `true` - Detect PII and show report, but don't anonymize or write to database
  - `false` - Actually anonymize data and write to database

### 5. Prepare Test Data

You'll need access to an OpenEHR server with test compositions containing PII. The configuration file specifies which server and database to use.

---

## Configuration

Atlas configuration is managed through a combination of TOML files and environment variables (following 12-factor app principles).

### Primary Configuration: TOML File

The main configuration is in `config/atlas.toml` (see Prerequisites section above for full example).

### Environment Variable Overrides

You can override specific settings using environment variables:

```bash
# OpenEHR credentials (recommended for secrets)
export ATLAS_OPENEHR_PASSWORD="your-password"

# Database credentials
export ATLAS_COSMOSDB_KEY="your-cosmos-key"
# or
export ATLAS_POSTGRESQL_PASSWORD="your-postgres-password"

# Anonymization settings (override TOML)
export ATLAS_ANONYMIZATION_ENABLED=true
export ATLAS_ANONYMIZATION_MODE=hipaa_safe_harbor  # or gdpr
export ATLAS_ANONYMIZATION_STRATEGY=token          # or redact
export ATLAS_ANONYMIZATION_DRY_RUN=false

# Audit logging
export ATLAS_ANONYMIZATION_AUDIT_ENABLED=true
export ATLAS_ANONYMIZATION_AUDIT_LOG_PATH=./audit/anonymization.log
export ATLAS_ANONYMIZATION_AUDIT_JSON_FORMAT=true
```

### CLI Argument Overrides

The export command supports these CLI arguments:

- `--template-id <ID>` - Override template ID(s) to export (comma-separated)
- `--ehr-id <ID>` - Override EHR ID(s) to export (comma-separated)
- `--mode <MODE>` - Override export mode (full or incremental)
- `--anonymize` - Enable anonymization (overrides config)
- `--anonymize-mode <MODE>` - Set compliance mode (gdpr or hipaa_safe_harbor)
- `--anonymize-dry-run` - Enable dry-run mode (detect only, don't anonymize)
- `--dry-run` - Dry run entire export (don't write to database)
- `--yes` or `-y` - Skip confirmation prompts

---

## Test Scenarios

### Scenario 1: Dry-Run Mode (Detection Only)

**Purpose**: Verify PII detection without modifying data.

**Steps**:

1. Configure `config/atlas.toml` with your OpenEHR and database settings (see Prerequisites).

2. Enable anonymization dry-run mode in the config:
   ```toml
   [anonymization]
   enabled = true
   mode = "hipaa_safe_harbor"
   strategy = "token"
   dry_run = true  # Enable dry-run
   ```

   Or use environment variable:
   ```bash
   export ATLAS_ANONYMIZATION_DRY_RUN=true
   ```

3. Run export with anonymization flags:
   ```bash
   ./target/release/atlas export \
     --template-id "HTN_Monitoring.v1" \
     --anonymize \
     --anonymize-dry-run
   ```

3. Check the console output for dry-run report:
   ```
   📊 ANONYMIZATION DRY-RUN REPORT
   ================================
   
   Total Compositions Processed: 100
   Total PII Detected: 450
   
   🔍 PII DETECTIONS BY CATEGORY:
   - Email: 100
   - Phone: 85
   - SSN: 50
   - Date: 150
   - Name: 65
   
   📝 SAMPLE ANONYMIZATIONS:
   1. Email: john.doe@example.com → TOKEN_EMAIL_a1b2c3d4
   2. Phone: (555) 123-4567 → TOKEN_PHONE_e5f6g7h8
   ...
   ```

**Expected Results**:
- PII should be detected and reported
- Original data should remain unchanged in the database
- Dry-run report should be displayed in console
- If audit logging is enabled, detections should be logged

---

### Scenario 2: HIPAA Safe Harbor Compliance

**Purpose**: Verify HIPAA Safe Harbor anonymization.

**Steps**:

1. Configure HIPAA mode in `atlas.toml`:
   ```toml
   [anonymization]
   enabled = true
   mode = "hipaa_safe_harbor"
   strategy = "token"
   dry_run = false  # Disable dry-run to actually anonymize
   ```

2. Run export:
   ```bash
   ./target/release/atlas export \
     --template-id "HTN_Monitoring.v1" \
     --anonymize \
     --anonymize-mode hipaa_safe_harbor
   ```

3. Query the database to verify anonymization:
   ```bash
   # For Cosmos DB
   az cosmosdb sql query \
     --account-name your-account \
     --database-name your-database \
     --container-name your-container \
     --query-text "SELECT * FROM c OFFSET 0 LIMIT 10"
   ```

**Expected Results**:
- All 18 HIPAA identifiers should be anonymized:
  1. Names → Redacted or tokenized
  2. Geographic subdivisions (addresses, zip codes) → Anonymized
  3. Dates (except year) → Anonymized
  4. Telephone numbers → Anonymized
  5. Fax numbers → Anonymized
  6. Email addresses → Anonymized
  7. Social Security Numbers → Anonymized
  8. Medical Record Numbers → Anonymized
  9. Health Plan Beneficiary Numbers → Anonymized
  10. Account Numbers → Anonymized
  11. Certificate/License Numbers → Anonymized
  12. Vehicle Identifiers → Anonymized
  13. Device Identifiers → Anonymized
  14. Web URLs → Anonymized
  15. IP Addresses → Anonymized
  16. Biometric Identifiers → Anonymized
  17. Full-face Photographs → Anonymized
  18. Other Unique Identifiers → Anonymized

---

### Scenario 3: GDPR Compliance

**Purpose**: Verify GDPR anonymization (includes HIPAA + quasi-identifiers).

**Steps**:

1. Configure GDPR mode in `atlas.toml`:
   ```toml
   [anonymization]
   enabled = true
   mode = "gdpr"
   strategy = "token"
   dry_run = false
   ```

2. Run export:
   ```bash
   ./target/release/atlas export \
     --template-id "HTN_Monitoring.v1" \
     --anonymize \
     --anonymize-mode gdpr
   ```

**Expected Results**:
- All HIPAA identifiers should be anonymized
- Additional GDPR quasi-identifiers should be detected and anonymized:
  - Occupation
  - Education level
  - Marital status
  - Ethnicity
  - Age (if specific)
  - Gender (in some contexts)

---

### Scenario 4: Redaction Strategy

**Purpose**: Verify redaction strategy replaces PII with `[REDACTED]`.

**Steps**:

1. Configure redaction strategy in `atlas.toml`:
   ```toml
   [anonymization]
   enabled = true
   mode = "hipaa_safe_harbor"
   strategy = "redact"  # Use redaction instead of tokens
   dry_run = true       # Use dry-run to see samples
   ```

2. Run export:
   ```bash
   ./target/release/atlas export \
     --template-id "HTN_Monitoring.v1" \
     --anonymize \
     --anonymize-dry-run
   ```

**Expected Results**:
- Dry-run report should show samples like:
  ```
  Email: john.doe@example.com → [REDACTED_EMAIL]
  Phone: (555) 123-4567 → [REDACTED_PHONE]
  SSN: 123-45-6789 → [REDACTED_SSN]
  ```

---

### Scenario 5: Tokenization Strategy

**Purpose**: Verify tokenization strategy replaces PII with random tokens.

**Steps**:

1. Configure tokenization strategy in `atlas.toml`:
   ```toml
   [anonymization]
   enabled = true
   mode = "hipaa_safe_harbor"
   strategy = "token"  # Use tokenization
   dry_run = true      # Use dry-run to see samples
   ```

2. Run export:
   ```bash
   ./target/release/atlas export \
     --template-id "HTN_Monitoring.v1" \
     --anonymize \
     --anonymize-dry-run
   ```

**Expected Results**:
- Dry-run report should show samples like:
  ```
  Email: john.doe@example.com → TOKEN_EMAIL_a1b2c3d4e5f6
  Phone: (555) 123-4567 → TOKEN_PHONE_g7h8i9j0k1l2
  SSN: 123-45-6789 → TOKEN_SSN_m3n4o5p6q7r8
  ```
- Tokens should be random and different each time

---

### Scenario 6: Audit Logging

**Purpose**: Verify audit logs are created correctly.

**Steps**:

1. Enable audit logging in `atlas.toml`:
   ```toml
   [anonymization]
   enabled = true
   mode = "hipaa_safe_harbor"
   strategy = "token"
   dry_run = false

   [anonymization.audit]
   enabled = true
   log_path = "./audit/test_audit.log"
   json_format = true
   ```

2. Create audit directory:
   ```bash
   mkdir -p ./audit
   ```

3. Run export:
   ```bash
   ./target/release/atlas export \
     --template-id "HTN_Monitoring.v1" \
     --anonymize
   ```

4. Check audit log:
   ```bash
   cat ./audit/test_audit.log | jq .
   ```

**Expected Results**:
- Audit log should contain JSON entries like:
  ```json
  {
    "timestamp": "2024-01-15T10:30:00Z",
    "composition_id": "550e8400-e29b-41d4-a716-446655440000",
    "pii_detected": 5,
    "categories": ["Email", "Phone", "SSN", "Date", "Name"],
    "strategy": "Token",
    "compliance_mode": "HipaaSafeHarbor",
    "pii_hash": "sha256:a1b2c3d4e5f6..."
  }
  ```
- Each anonymized composition should have an audit entry
- PII values should be hashed (SHA-256), not stored in plaintext

---

## Verification Steps

### 1. Verify PII Detection Accuracy

**Recall Test** (False Negatives):
- Create test compositions with known PII
- Run in dry-run mode
- Verify all PII is detected
- Target: ≥98% recall

**Precision Test** (False Positives):
- Create test compositions with non-PII data
- Run in dry-run mode
- Verify minimal false positives
- Target: ≥95% precision

### 2. Verify Data Anonymization

Query the database and verify:
- Original PII values are not present
- Anonymized values are in expected format
- Non-PII data is unchanged
- JSON structure is preserved

### 3. Verify Compliance Modes

**HIPAA Test**:
```bash
# Should detect 18 HIPAA identifier types
./target/release/atlas export \
  --template-id "HTN_Monitoring.v1" \
  --anonymize \
  --anonymize-mode hipaa_safe_harbor \
  --anonymize-dry-run
```

**GDPR Test**:
```bash
# Should detect HIPAA + quasi-identifiers
./target/release/atlas export \
  --template-id "HTN_Monitoring.v1" \
  --anonymize \
  --anonymize-mode gdpr \
  --anonymize-dry-run
```

Compare detection counts - GDPR should detect ≥ HIPAA.

---

## Performance Testing

### Baseline Performance (Without Anonymization)

1. Disable anonymization in `config/atlas.toml`:
   ```toml
   [anonymization]
   enabled = false
   ```

2. Run export and measure:
   ```bash
   time ./target/release/atlas export \
     --template-id "HTN_Monitoring.v1"
   ```

Record:
- Total time
- Compositions/second
- Memory usage

### Anonymization Performance

1. Enable anonymization in `atlas.toml`:
   ```toml
   [anonymization]
   enabled = true
   mode = "hipaa_safe_harbor"
   strategy = "token"
   dry_run = false
   ```

2. Run export and measure:
   ```bash
   time ./target/release/atlas export \
     --template-id "HTN_Monitoring.v1" \
     --anonymize
   ```

Record:
- Total time
- Compositions/second
- Memory usage

### Performance Requirements

- **Overhead**: <100ms per composition
- **Throughput Impact**: <15% reduction
- **Memory**: No significant increase

### Verification

```bash
# Calculate overhead
overhead_ms = (anonymization_time - baseline_time) / composition_count

# Calculate throughput impact
throughput_impact = ((baseline_throughput - anonymization_throughput) / baseline_throughput) * 100

# Verify requirements
echo "Overhead: ${overhead_ms}ms (should be <100ms)"
echo "Throughput impact: ${throughput_impact}% (should be <15%)"
```

---

## Troubleshooting

### Issue: No PII Detected

**Possible Causes**:
- Anonymization not enabled
- Pattern library not loaded
- Data doesn't contain PII

**Solutions**:
1. Verify configuration:
   ```bash
   echo $ATLAS_ANONYMIZATION_ENABLED  # Should be "true"
   ```
2. Check logs for pattern library loading
3. Test with known PII data

### Issue: Performance Degradation

**Possible Causes**:
- Large batch sizes
- Complex regex patterns
- Insufficient resources

**Solutions**:
1. Reduce batch size
2. Increase memory allocation
3. Profile with `cargo flamegraph`

### Issue: Audit Log Not Created

**Possible Causes**:
- Audit logging not enabled
- Invalid log path
- Permission issues

**Solutions**:
1. Verify audit configuration
2. Create audit directory: `mkdir -p ./audit`
3. Check file permissions

### Issue: Incorrect Anonymization

**Possible Causes**:
- Wrong strategy configured
- Pattern mismatch
- Nested data structure issues

**Solutions**:
1. Verify strategy configuration
2. Run in dry-run mode to see detections
3. Check pattern library for coverage

---

## Summary Checklist

- [ ] All tests pass (207 lib + 41 integration + 56 doctests)
- [ ] Dry-run mode works and displays report
- [ ] HIPAA mode detects all 18 identifier types
- [ ] GDPR mode detects HIPAA + quasi-identifiers
- [ ] Redaction strategy works correctly
- [ ] Tokenization strategy works correctly
- [ ] Audit logging creates correct entries
- [ ] Performance overhead is <100ms per composition
- [ ] Throughput impact is <15%
- [ ] No PII leaks to database
- [ ] Non-PII data is preserved
- [ ] JSON structure is maintained

---

## Next Steps

After completing manual testing:

1. **Document Results**: Record all test results and any issues found
2. **Performance Benchmarks**: Create formal benchmark suite (Task 16)
3. **Documentation**: Write user guide and API documentation (Task 17)
4. **Final Validation**: Review against acceptance criteria (Task 18)
5. **Create Pull Request**: Submit for code review
6. **Production Deployment**: Plan rollout strategy

---

## Support

For issues or questions:
- Check the test suite: `tests/anonymization_*.rs`
- Review the PRD: `.prd/anonymization.md`
- Check progress: `ANONYMIZATION_PROGRESS.md`

