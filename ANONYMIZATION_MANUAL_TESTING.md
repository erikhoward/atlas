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

### 3. Prepare Test Data

You'll need access to an OpenEHR server with test compositions containing PII. Alternatively, you can use the synthetic data from the integration tests.

---

## Configuration

### Option 1: Environment Variables

Set the following environment variables:

```bash
# Enable anonymization
export ATLAS_ANONYMIZATION_ENABLED=true

# Set compliance mode (HipaaSafeHarbor or Gdpr)
export ATLAS_ANONYMIZATION_MODE=HipaaSafeHarbor

# Set anonymization strategy (Redact or Token)
export ATLAS_ANONYMIZATION_STRATEGY=Token

# Enable dry-run mode (optional - for testing without anonymizing)
export ATLAS_ANONYMIZATION_DRY_RUN=false

# Enable audit logging (optional)
export ATLAS_ANONYMIZATION_AUDIT_ENABLED=true
export ATLAS_ANONYMIZATION_AUDIT_LOG_PATH=./audit/anonymization.log
export ATLAS_ANONYMIZATION_AUDIT_JSON_FORMAT=true
```

### Option 2: TOML Configuration File

Create or edit `config/atlas.toml`:

```toml
[anonymization]
enabled = true
mode = "HipaaSafeHarbor"  # or "Gdpr"
strategy = "Token"         # or "Redact"
dry_run = false

[anonymization.audit]
enabled = true
log_path = "./audit/anonymization.log"
json_format = true
```

---

## Test Scenarios

### Scenario 1: Dry-Run Mode (Detection Only)

**Purpose**: Verify PII detection without modifying data.

**Steps**:

1. Enable dry-run mode:
   ```bash
   export ATLAS_ANONYMIZATION_DRY_RUN=true
   ```

2. Run export with anonymization flags:
   ```bash
   ./target/release/atlas export \
     --openehr-url "https://your-openehr-server.com" \
     --template-id "your.template.v1" \
     --database-type cosmosdb \
     --anonymize \
     --anonymize-mode hipaa \
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

1. Configure HIPAA mode:
   ```bash
   export ATLAS_ANONYMIZATION_MODE=HipaaSafeHarbor
   export ATLAS_ANONYMIZATION_DRY_RUN=false
   ```

2. Run export:
   ```bash
   ./target/release/atlas export \
     --openehr-url "https://your-openehr-server.com" \
     --template-id "your.template.v1" \
     --database-type cosmosdb \
     --anonymize \
     --anonymize-mode hipaa
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

1. Configure GDPR mode:
   ```bash
   export ATLAS_ANONYMIZATION_MODE=Gdpr
   export ATLAS_ANONYMIZATION_DRY_RUN=false
   ```

2. Run export:
   ```bash
   ./target/release/atlas export \
     --openehr-url "https://your-openehr-server.com" \
     --template-id "your.template.v1" \
     --database-type postgresql \
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

1. Configure redaction strategy:
   ```bash
   export ATLAS_ANONYMIZATION_STRATEGY=Redact
   ```

2. Run export with dry-run to see samples:
   ```bash
   ./target/release/atlas export \
     --openehr-url "https://your-openehr-server.com" \
     --template-id "your.template.v1" \
     --database-type cosmosdb \
     --anonymize \
     --anonymize-strategy redact \
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

1. Configure tokenization strategy:
   ```bash
   export ATLAS_ANONYMIZATION_STRATEGY=Token
   ```

2. Run export with dry-run:
   ```bash
   ./target/release/atlas export \
     --openehr-url "https://your-openehr-server.com" \
     --template-id "your.template.v1" \
     --database-type cosmosdb \
     --anonymize \
     --anonymize-strategy token \
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

1. Enable audit logging:
   ```bash
   export ATLAS_ANONYMIZATION_AUDIT_ENABLED=true
   export ATLAS_ANONYMIZATION_AUDIT_LOG_PATH=./audit/test_audit.log
   export ATLAS_ANONYMIZATION_AUDIT_JSON_FORMAT=true
   ```

2. Create audit directory:
   ```bash
   mkdir -p ./audit
   ```

3. Run export:
   ```bash
   ./target/release/atlas export \
     --openehr-url "https://your-openehr-server.com" \
     --template-id "your.template.v1" \
     --database-type cosmosdb \
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
./target/release/atlas export --anonymize --anonymize-mode hipaa --anonymize-dry-run
```

**GDPR Test**:
```bash
# Should detect HIPAA + quasi-identifiers
./target/release/atlas export --anonymize --anonymize-mode gdpr --anonymize-dry-run
```

Compare detection counts - GDPR should detect ≥ HIPAA.

---

## Performance Testing

### Baseline Performance (Without Anonymization)

```bash
time ./target/release/atlas export \
  --openehr-url "https://your-openehr-server.com" \
  --template-id "your.template.v1" \
  --database-type cosmosdb
```

Record:
- Total time
- Compositions/second
- Memory usage

### Anonymization Performance

```bash
time ./target/release/atlas export \
  --openehr-url "https://your-openehr-server.com" \
  --template-id "your.template.v1" \
  --database-type cosmosdb \
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

