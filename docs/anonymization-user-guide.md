# Anonymization User Guide

## Overview

Atlas provides built-in anonymization capabilities to protect Protected Health Information (PHI) and Personally Identifiable Information (PII) when exporting OpenEHR compositions to databases. This feature helps organizations comply with healthcare data privacy regulations such as HIPAA (US) and GDPR (EU).

### Key Features

- **Automated PII Detection**: Regex-based detection of 24+ PII categories
- **Multiple Compliance Modes**: HIPAA Safe Harbor and GDPR support
- **Flexible Anonymization Strategies**: Redaction or tokenization
- **Dry-Run Mode**: Preview PII detection without modifying data
- **Comprehensive Audit Logging**: Track all anonymization operations with SHA-256 hashed values
- **Zero Performance Impact**: <100ms overhead per composition, <15% throughput impact

---

## Quick Start

### 1. Enable Anonymization

Add the following section to your `atlas.toml`:

```toml
[anonymization]
enabled = true
mode = "hipaa_safe_harbor"  # or "gdpr"
strategy = "token"          # or "redact"
dry_run = false

[anonymization.audit]
enabled = true
log_path = "./audit/anonymization.log"
json_format = true
```

### 2. Create Audit Directory

```bash
mkdir -p ./audit
```

### 3. Run Export with Anonymization

```bash
atlas export --template-id "YourTemplate.v1" --anonymize
```

---

## Configuration

### TOML Configuration

Atlas supports configuration via `atlas.toml` file:

```toml
[anonymization]
# Enable/disable anonymization (default: false)
enabled = true

# Compliance mode: "hipaa_safe_harbor" or "gdpr" (default: "gdpr")
mode = "hipaa_safe_harbor"

# Anonymization strategy: "token" or "redact" (default: "token")
strategy = "token"

# Dry-run mode: detect PII without anonymizing (default: false)
dry_run = false

# Optional: Custom pattern library path
# pattern_library = "./patterns/custom_pii_patterns.toml"

[anonymization.audit]
# Enable audit logging (default: true)
enabled = true

# Audit log file path (default: "./audit/anonymization.log")
log_path = "./audit/anonymization.log"

# Use JSON format for audit logs (default: true)
json_format = true
```

### Environment Variables

Override configuration using environment variables (12-factor app pattern):

```bash
# Core settings
export ATLAS_ANONYMIZATION_ENABLED=true
export ATLAS_ANONYMIZATION_MODE=hipaa_safe_harbor  # or gdpr
export ATLAS_ANONYMIZATION_STRATEGY=token          # or redact
export ATLAS_ANONYMIZATION_DRY_RUN=false

# Audit settings
export ATLAS_ANONYMIZATION_AUDIT_ENABLED=true
export ATLAS_ANONYMIZATION_AUDIT_LOG_PATH=./audit/anonymization.log
export ATLAS_ANONYMIZATION_AUDIT_JSON_FORMAT=true

# Optional: Custom pattern library
export ATLAS_ANONYMIZATION_PATTERN_LIBRARY=./patterns/custom_pii_patterns.toml
```

**Precedence:** CLI flags > Environment variables > TOML configuration

---

## CLI Usage

### Basic Commands

```bash
# Enable anonymization (uses config file settings)
atlas export --template-id "Template.v1" --anonymize

# Override compliance mode
atlas export --template-id "Template.v1" --anonymize --anonymize-mode gdpr

# Dry-run mode (detect PII without anonymizing)
atlas export --template-id "Template.v1" --anonymize --anonymize-dry-run

# Skip confirmation prompts
atlas export --template-id "Template.v1" --anonymize -y
```

### CLI Flags

| Flag | Description | Values |
|------|-------------|--------|
| `--anonymize` | Enable anonymization | (boolean flag) |
| `--anonymize-mode <MODE>` | Set compliance mode | `hipaa_safe_harbor`, `gdpr` |
| `--anonymize-dry-run` | Enable dry-run mode | (boolean flag) |
| `-y`, `--yes` | Skip confirmation prompts | (boolean flag) |

---

## Compliance Modes

### HIPAA Safe Harbor Mode

**Mode:** `hipaa_safe_harbor`

Detects and anonymizes the 18 identifiers specified in the HIPAA Safe Harbor method (45 CFR §164.514(b)(2)):

1. **Names** - Patient names, family members, employers
2. **Geographic Locations** - Addresses, cities, counties, ZIP codes (first 3 digits retained if >20,000 people)
3. **Dates** - Birth dates, admission dates, discharge dates, death dates (year may be retained)
4. **Telephone Numbers** - All phone/fax numbers
5. **Email Addresses** - All email addresses
6. **Social Security Numbers** - SSNs and national identifiers
7. **Medical Record Numbers** - MRNs and account numbers
8. **Health Plan Numbers** - Insurance and beneficiary numbers
9. **Certificate/License Numbers** - Professional licenses, certificates
10. **Vehicle Identifiers** - License plates, VINs
11. **Device Identifiers** - Serial numbers, device IDs
12. **URLs** - Web addresses
13. **IP Addresses** - Internet protocol addresses
14. **Biometric Identifiers** - Fingerprints, retinal scans, voice prints
15. **Photographic Images** - Full-face photos and comparable images
16. **Unique Identifying Numbers** - Any other unique identifying number, characteristic, or code

**Use Case:** US healthcare organizations subject to HIPAA regulations.

### GDPR Mode

**Mode:** `gdpr`

Detects all HIPAA identifiers PLUS additional GDPR quasi-identifiers:

17. **Occupation** - Job titles and employment information
18. **Education Level** - Educational background
19. **Marital Status** - Relationship status
20. **Ethnicity/Race** - Ethnic and racial information
21. **Age** - Specific age values (may be generalized to ranges)
22. **Gender** - Gender identity information

**Use Case:** European healthcare organizations subject to GDPR, or organizations serving both US and EU patients.

---

## Anonymization Strategies

### Token Strategy (Recommended)

**Strategy:** `token`

Replaces PII with unique random tokens that maintain referential integrity within a single export run.

**Example:**
```
Original: "Patient: John Doe, Email: john.doe@example.com"
Anonymized: "Patient: TOKEN_NAME_a1b2c3d4, Email: TOKEN_EMAIL_e5f6g7h8"
```

**Advantages:**
- Maintains data relationships within the same export
- Enables limited analytics on anonymized data
- Unique tokens per PII value per run

**Use Cases:**
- Research datasets
- Analytics and reporting
- Data sharing with third parties

### Redact Strategy

**Strategy:** `redact`

Replaces PII with category-specific redaction markers.

**Example:**
```
Original: "Patient: John Doe, Email: john.doe@example.com"
Anonymized: "Patient: [REDACTED_NAME], Email: [REDACTED_EMAIL]"
```

**Advantages:**
- Clear indication of removed data
- Simpler implementation
- Easier to audit

**Use Cases:**
- Compliance audits
- Legal discovery
- Maximum privacy protection

---

## Dry-Run Mode

Dry-run mode detects PII and generates a report **without** anonymizing data or writing to the database.

### Enable Dry-Run

**Via Configuration:**
```toml
[anonymization]
dry_run = true
```

**Via CLI:**
```bash
atlas export --template-id "Template.v1" --anonymize --anonymize-dry-run
```

### Dry-Run Report

The dry-run report includes:

- **Summary Statistics**: Total compositions, total PII detected
- **Detections by Category**: Breakdown of PII types found
- **Sample Detections**: Example PII values (first 10)
- **Warnings**: Potential issues or edge cases
- **Processing Stats**: Performance metrics

**Example Output:**
```
📊 ANONYMIZATION DRY-RUN REPORT
════════════════════════════════════════════════════════════════

Total Compositions Analyzed: 150
Total PII Detections: 1,247

🔍 DETECTIONS BY CATEGORY
────────────────────────────────────────────────────────────────
  Name:                    150 occurrences
  Email:                   145 occurrences
  PhoneNumber:             142 occurrences
  Date:                    380 occurrences
  MedicalRecordNumber:     150 occurrences
  SocialSecurityNumber:     75 occurrences
  GeographicLocation:      205 occurrences

📝 SAMPLE DETECTIONS (first 10)
────────────────────────────────────────────────────────────────
  [Name] "John Doe" at composition_1/patient/name
  [Email] "john.doe@example.com" at composition_1/patient/email
  [PhoneNumber] "+1-555-123-4567" at composition_1/patient/phone
  ...

⚠️  WARNINGS
────────────────────────────────────────────────────────────────
  No warnings

⏱️  PROCESSING STATS
────────────────────────────────────────────────────────────────
  Total Processing Time: 2.5s
  Average Time per Composition: 16.7ms
  Throughput: 60 compositions/second
```

---

## Audit Logging

### Audit Log Format

Audit logs track all anonymization operations with the following information:

- **Timestamp**: When the anonymization occurred
- **Composition ID**: Which composition was processed
- **Detections**: List of PII categories detected
- **Original Value Hash**: SHA-256 hash of original PII (never plaintext)
- **Anonymized Value**: The replacement value
- **Strategy**: Which anonymization strategy was used
- **Processing Time**: How long the operation took

**Example JSON Audit Log:**
```json
{
  "timestamp": "2025-11-12T10:30:45.123Z",
  "composition_id": "84d7c3f5::local.ehrbase.org::1",
  "detections": [
    {
      "category": "Name",
      "original_hash": "5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8",
      "anonymized_value": "TOKEN_NAME_a1b2c3d4",
      "position": "patient/name"
    }
  ],
  "strategy": "token",
  "processing_time_ms": 15
}
```

### Security Considerations

- **Never logs plaintext PII**: Original values are hashed with SHA-256
- **Secure file permissions**: Audit logs should have restricted access (600)
- **Rotation recommended**: Implement log rotation for long-running deployments

---

## Best Practices

### 1. Always Test with Dry-Run First

Before anonymizing production data, run in dry-run mode to:
- Verify PII detection accuracy
- Review sample detections
- Estimate performance impact

```bash
atlas export --template-id "Template.v1" --anonymize --anonymize-dry-run
```

### 2. Choose the Right Compliance Mode

- **HIPAA Safe Harbor**: US healthcare organizations
- **GDPR**: European organizations or multi-region deployments

### 3. Select Appropriate Strategy

- **Token**: For research, analytics, maintaining relationships
- **Redact**: For maximum privacy, compliance audits

### 4. Enable Audit Logging

Always enable audit logging for:
- Compliance verification
- Troubleshooting
- Performance monitoring

### 5. Secure Audit Logs

```bash
# Set restrictive permissions
chmod 600 ./audit/anonymization.log

# Implement log rotation
logrotate /etc/logrotate.d/atlas-anonymization
```

### 6. Monitor Performance

Review audit logs for processing times:
- Target: <100ms per composition
- Alert if >200ms consistently

### 7. Validate Anonymization

Periodically review anonymized data to ensure:
- No PII leakage
- Data utility preserved
- Compliance requirements met

---

## Troubleshooting

### Issue: "unknown variant" TOML Parse Error

**Error:**
```
TOML parse error: unknown variant `HipaaSafeHarbor`, expected `gdpr` or `hipaa_safe_harbor`
```

**Solution:** Use lowercase with underscores:
```toml
mode = "hipaa_safe_harbor"  # NOT "HipaaSafeHarbor"
strategy = "token"          # NOT "Token"
```

### Issue: No PII Detected

**Possible Causes:**
1. Data doesn't contain PII
2. PII format not recognized by regex patterns
3. Wrong compliance mode selected

**Solutions:**
- Review dry-run report samples
- Check if data format matches expected patterns
- Consider custom pattern library for domain-specific formats

### Issue: Performance Degradation

**Symptoms:** Export takes significantly longer with anonymization enabled

**Solutions:**
1. Check audit logs for per-composition processing times
2. Reduce batch size if memory constrained
3. Review regex patterns for inefficiencies
4. Consider disabling audit logging for maximum performance

### Issue: Audit Log Not Created

**Possible Causes:**
1. Audit directory doesn't exist
2. Insufficient permissions
3. Audit logging disabled in config

**Solutions:**
```bash
# Create audit directory
mkdir -p ./audit

# Set permissions
chmod 700 ./audit

# Verify configuration
[anonymization.audit]
enabled = true
log_path = "./audit/anonymization.log"
```

---

## Advanced Topics

### Custom Pattern Library

Create a custom TOML file with additional PII patterns:

```toml
# custom_patterns.toml
[[patterns]]
category = "MedicalRecordNumber"
pattern = "MRN-\\d{8}"
description = "Custom MRN format"

[[patterns]]
category = "Name"
pattern = "Dr\\. [A-Z][a-z]+ [A-Z][a-z]+"
description = "Physician names"
```

Configure Atlas to use it:
```toml
[anonymization]
pattern_library = "./patterns/custom_patterns.toml"
```

### Integration with CI/CD

```yaml
# .github/workflows/export.yml
- name: Export with Anonymization
  run: |
    atlas export \
      --template-id "Template.v1" \
      --anonymize \
      --anonymize-mode hipaa_safe_harbor \
      -y
  env:
    ATLAS_ANONYMIZATION_ENABLED: true
    ATLAS_ANONYMIZATION_AUDIT_LOG_PATH: ./audit/ci-anonymization.log
```

---

## FAQ

**Q: Does anonymization work with both preserve and flatten modes?**  
A: Yes, anonymization works with both `preserve` and `flatten` composition formats.

**Q: Can I anonymize existing data in the database?**  
A: No, Phase 1 only anonymizes during export. Re-export with anonymization enabled to create anonymized copies.

**Q: Are tokens consistent across multiple export runs?**  
A: No, tokens are randomly generated per export run. The same PII value will get different tokens in different runs.

**Q: Can I customize which PII categories to detect?**  
A: Phase 1 uses fixed category sets per compliance mode. Custom category selection is planned for Phase 2.

**Q: What happens if anonymization fails for a composition?**  
A: The composition is skipped, an error is logged, and the export continues with remaining compositions.

**Q: Does dry-run mode write to the database?**  
A: No, dry-run mode only detects PII and generates a report. No data is written to the database.

---

## Support

For issues, questions, or feature requests:
- **GitHub Issues**: https://github.com/erikhoward/atlas/issues
- **Documentation**: https://github.com/erikhoward/atlas/tree/main/docs
- **PRD**: `.prd/anonymization.md`

---

**Last Updated:** 2025-11-12  
**Version:** Phase 1 (v2.2.0)

