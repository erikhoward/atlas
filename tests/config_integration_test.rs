//! Integration tests for configuration loading and validation
//!
//! Note: Tests that modify environment variables should be run with --test-threads=1
//! to avoid interference between tests.

use atlas::config::load_config;
use std::io::Write;
use std::sync::Mutex;
use tempfile::NamedTempFile;

// Mutex to serialize tests that modify environment variables
static ENV_MUTEX: Mutex<()> = Mutex::new(());

/// Helper function to clean up environment variables
fn cleanup_env_vars() {
    std::env::remove_var("ATLAS_APPLICATION_LOG_LEVEL");
    std::env::remove_var("ATLAS_APPLICATION_DRY_RUN");
    std::env::remove_var("ATLAS_EXPORT_MODE");
    std::env::remove_var("ATLAS_OPENEHR_QUERY_BATCH_SIZE");
    std::env::remove_var("ATLAS_OPENEHR_QUERY_PARALLEL_EHRS");
    std::env::remove_var("TEST_OPENEHR_PASSWORD");
    std::env::remove_var("TEST_COSMOS_KEY");
}

#[test]
fn test_load_complete_config() {
    cleanup_env_vars();
    let toml_content = r#"
[application]
name = "atlas"
version = "1.0.0"
log_level = "debug"
dry_run = true

[openehr]
base_url = "https://ehrbase.example.com/ehrbase/rest/openehr/v1"
vendor = "ehrbase"
auth_type = "basic"
username = "test_user"
password = "test_pass"
tls_verify = true

[openehr.query]
template_ids = ["IDCR - Lab Report.v1", "IDCR - Vital Signs.v1"]
ehr_ids = ["ehr-123", "ehr-456"]
time_range_start = "2024-01-01T00:00:00Z"
batch_size = 500
parallel_ehrs = 4

[export]
mode = "full"
export_composition_format = "flatten"
max_retries = 5
retry_backoff_ms = [500, 1000, 2000]

[cosmosdb]
endpoint = "https://test.documents.azure.com:443/"
key = "test-key-12345"
database_name = "test_openehr"
control_container = "test_control"
data_container_prefix = "test_compositions"
partition_key = "/ehr_id"
max_concurrency = 20
request_timeout_seconds = 120

[state]
enable_checkpointing = false
checkpoint_interval_seconds = 60

[verification]
enable_verification = true
checksum_algorithm = "sha512"

[logging]
local_enabled = false
local_path = "/tmp/atlas"
local_rotation = "size"
local_max_size_mb = 50
azure_enabled = false
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(toml_content.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let config = load_config(temp_file.path()).expect("Failed to load config");

    // Verify application config
    assert_eq!(config.application.name, "atlas");
    assert_eq!(config.application.version, "1.0.0");
    assert_eq!(config.application.log_level, "debug");
    assert!(config.application.dry_run);

    // Verify OpenEHR config
    assert_eq!(
        config.openehr.base_url,
        "https://ehrbase.example.com/ehrbase/rest/openehr/v1"
    );
    assert_eq!(config.openehr.vendor, "ehrbase");
    assert_eq!(config.openehr.username, Some("test_user".to_string()));
    assert_eq!(config.openehr.password, Some("test_pass".to_string()));

    // Verify query config
    assert_eq!(config.openehr.query.template_ids.len(), 2);
    assert_eq!(config.openehr.query.ehr_ids.len(), 2);
    assert_eq!(config.openehr.query.batch_size, 500);
    assert_eq!(config.openehr.query.parallel_ehrs, 4);

    // Verify export config
    assert_eq!(config.export.mode, "full");
    assert_eq!(config.export.export_composition_format, "flatten");
    assert_eq!(config.export.max_retries, 5);

    // Verify Cosmos DB config
    assert_eq!(
        config.cosmosdb.endpoint,
        "https://test.documents.azure.com:443/"
    );
    assert_eq!(config.cosmosdb.database_name, "test_openehr");
    assert_eq!(config.cosmosdb.max_concurrency, 20);

    // Verify state config
    assert!(!config.state.enable_checkpointing);
    assert_eq!(config.state.checkpoint_interval_seconds, 60);

    // Verify verification config
    assert!(config.verification.enable_verification);
    assert_eq!(config.verification.checksum_algorithm, "sha512");

    // Verify logging config
    assert!(!config.logging.local_enabled);
    assert_eq!(config.logging.local_path, "/tmp/atlas");
    assert_eq!(config.logging.local_rotation, "size");
}

#[test]
fn test_load_minimal_config_with_defaults() {
    cleanup_env_vars();

    let toml_content = r#"
[application]
name = "atlas"
version = "1.0.0"

[openehr]
base_url = "https://ehrbase.example.com"
username = "user"
password = "pass"

[openehr.query]
template_ids = ["template1"]

[export]

[cosmosdb]
endpoint = "https://test.documents.azure.com:443/"
key = "test-key"
database_name = "test_db"

[state]
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(toml_content.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let config = load_config(temp_file.path()).expect("Failed to load config");

    // Verify defaults are applied
    assert_eq!(config.application.log_level, "info");
    assert!(!config.application.dry_run);
    assert_eq!(config.openehr.vendor, "ehrbase");
    assert_eq!(config.openehr.auth_type, "basic");
    assert_eq!(config.openehr.query.batch_size, 1000);
    assert_eq!(config.openehr.query.parallel_ehrs, 8);
    assert_eq!(config.export.mode, "incremental");
    assert_eq!(config.export.export_composition_format, "preserve");
    assert_eq!(config.export.max_retries, 3);
    assert_eq!(config.cosmosdb.control_container, "atlas_control");
    assert_eq!(config.cosmosdb.data_container_prefix, "compositions");
    assert!(config.state.enable_checkpointing);
    assert_eq!(config.state.checkpoint_interval_seconds, 30);
}

#[test]
fn test_env_var_substitution() {
    let _lock = ENV_MUTEX.lock().unwrap();
    cleanup_env_vars();
    std::env::set_var("TEST_OPENEHR_PASSWORD", "secret_pass");
    std::env::set_var("TEST_COSMOS_KEY", "secret_key");

    let toml_content = r#"
[application]
name = "atlas"
version = "1.0.0"

[openehr]
base_url = "https://ehrbase.example.com"
username = "user"
password = "${TEST_OPENEHR_PASSWORD}"

[openehr.query]
template_ids = ["template1"]

[export]

[cosmosdb]
endpoint = "https://test.documents.azure.com:443/"
key = "${TEST_COSMOS_KEY}"
database_name = "test_db"

[state]
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(toml_content.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let config = load_config(temp_file.path()).expect("Failed to load config");

    assert_eq!(config.openehr.password, Some("secret_pass".to_string()));
    assert_eq!(config.cosmosdb.key, "secret_key");

    std::env::remove_var("TEST_OPENEHR_PASSWORD");
    std::env::remove_var("TEST_COSMOS_KEY");
}

#[test]
fn test_env_var_overrides() {
    let _lock = ENV_MUTEX.lock().unwrap();
    cleanup_env_vars();
    std::env::set_var("ATLAS_APPLICATION_LOG_LEVEL", "trace");
    std::env::set_var("ATLAS_EXPORT_MODE", "full");
    std::env::set_var("ATLAS_OPENEHR_QUERY_BATCH_SIZE", "2000");

    let toml_content = r#"
[application]
name = "atlas"
version = "1.0.0"
log_level = "info"

[openehr]
base_url = "https://ehrbase.example.com"
username = "user"
password = "pass"

[openehr.query]
template_ids = ["template1"]
batch_size = 500

[export]
mode = "incremental"

[cosmosdb]
endpoint = "https://test.documents.azure.com:443/"
key = "test-key"
database_name = "test_db"

[state]
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(toml_content.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let config = load_config(temp_file.path()).expect("Failed to load config");

    // Verify env var overrides took effect
    assert_eq!(config.application.log_level, "trace");
    assert_eq!(config.export.mode, "full");
    assert_eq!(config.openehr.query.batch_size, 2000);

    std::env::remove_var("ATLAS_APPLICATION_LOG_LEVEL");
    std::env::remove_var("ATLAS_EXPORT_MODE");
    std::env::remove_var("ATLAS_OPENEHR_QUERY_BATCH_SIZE");
}

#[test]
fn test_invalid_config_validation() {
    cleanup_env_vars();

    let toml_content = r#"
[application]
name = "atlas"
version = "1.0.0"
log_level = "invalid_level"

[openehr]
base_url = "https://ehrbase.example.com"
username = "user"
password = "pass"

[openehr.query]
template_ids = ["template1"]

[export]

[cosmosdb]
endpoint = "https://test.documents.azure.com:443/"
key = "test-key"
database_name = "test_db"

[state]
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(toml_content.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let result = load_config(temp_file.path());
    assert!(result.is_err());
}
