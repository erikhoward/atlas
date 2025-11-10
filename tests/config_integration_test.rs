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
    std::env::remove_var("ATLAS_ENVIRONMENT");
    std::env::remove_var("ATLAS_DATABASE_TARGET");
    std::env::remove_var("ATLAS_APPLICATION_NAME");
    std::env::remove_var("ATLAS_APPLICATION_VERSION");
    std::env::remove_var("ATLAS_APPLICATION_LOG_LEVEL");
    std::env::remove_var("ATLAS_APPLICATION_DRY_RUN");
    std::env::remove_var("ATLAS_OPENEHR_BASE_URL");
    std::env::remove_var("ATLAS_OPENEHR_USERNAME");
    std::env::remove_var("ATLAS_OPENEHR_PASSWORD");
    std::env::remove_var("ATLAS_OPENEHR_TIMEOUT_SECONDS");
    std::env::remove_var("ATLAS_OPENEHR_TLS_VERIFY_CERTIFICATES");
    std::env::remove_var("ATLAS_OPENEHR_TLS_CA_CERT");
    std::env::remove_var("ATLAS_OPENEHR_RETRY_MAX_RETRIES");
    std::env::remove_var("ATLAS_OPENEHR_QUERY_TEMPLATE_IDS");
    std::env::remove_var("ATLAS_OPENEHR_QUERY_EHR_IDS");
    std::env::remove_var("ATLAS_OPENEHR_QUERY_BATCH_SIZE");
    std::env::remove_var("ATLAS_OPENEHR_QUERY_PARALLEL_EHRS");
    std::env::remove_var("ATLAS_EXPORT_MODE");
    std::env::remove_var("ATLAS_EXPORT_DRY_RUN");
    std::env::remove_var("ATLAS_EXPORT_RETRY_BACKOFF_MS");
    std::env::remove_var("ATLAS_EXPORT_SHUTDOWN_TIMEOUT_SECS");
    std::env::remove_var("ATLAS_COSMOSDB_PARTITION_KEY");
    std::env::remove_var("ATLAS_COSMOSDB_REQUEST_TIMEOUT_SECONDS");
    std::env::remove_var("ATLAS_POSTGRESQL_CONNECTION_STRING");
    std::env::remove_var("ATLAS_POSTGRESQL_MAX_CONNECTIONS");
    std::env::remove_var("ATLAS_POSTGRESQL_SSL_MODE");
    std::env::remove_var("ATLAS_LOGGING_LOCAL_ROTATION");
    std::env::remove_var("ATLAS_LOGGING_LOCAL_MAX_SIZE_MB");
    std::env::remove_var("TEST_OPENEHR_PASSWORD");
    std::env::remove_var("TEST_COSMOS_KEY");
}

#[test]
fn test_load_complete_config() {
    cleanup_env_vars();
    let toml_content = r#"database_target = "cosmosdb"
environment = "development"

[application]
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
    assert_eq!(config.application.log_level, "debug");
    assert!(config.application.dry_run);

    // Verify OpenEHR config
    assert_eq!(
        config.openehr.base_url,
        "https://ehrbase.example.com/ehrbase/rest/openehr/v1"
    );
    assert_eq!(config.openehr.vendor, "ehrbase");
    assert_eq!(config.openehr.username, Some("test_user".to_string()));

    // Verify password (using expose_secret to access the protected value)
    use secrecy::ExposeSecret;
    assert_eq!(
        config
            .openehr
            .password
            .as_ref()
            .map(|s| s.expose_secret().as_ref()),
        Some("test_pass")
    );

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
    let cosmosdb = config
        .cosmosdb
        .as_ref()
        .expect("CosmosDB config should be present");
    assert_eq!(cosmosdb.endpoint, "https://test.documents.azure.com:443/");
    assert_eq!(cosmosdb.database_name, "test_openehr");
    assert_eq!(cosmosdb.max_concurrency, 20);

    // Verify state config
    assert!(!config.state.enable_checkpointing);
    assert_eq!(config.state.checkpoint_interval_seconds, 60);

    // Verify verification config
    assert!(config.verification.enable_verification);

    // Verify logging config
    assert!(!config.logging.local_enabled);
    assert_eq!(config.logging.local_path, "/tmp/atlas");
    assert_eq!(config.logging.local_rotation, "size");
}

#[test]
fn test_load_minimal_config_with_defaults() {
    cleanup_env_vars();

    let toml_content = r#"database_target = "cosmosdb"
environment = "development"

[application]

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
    let cosmosdb = config
        .cosmosdb
        .as_ref()
        .expect("CosmosDB config should be present");
    assert_eq!(cosmosdb.control_container, "atlas_control");
    assert_eq!(cosmosdb.data_container_prefix, "compositions");
    assert!(config.state.enable_checkpointing);
    assert_eq!(config.state.checkpoint_interval_seconds, 30);
}

#[test]
fn test_env_var_substitution() {
    let _lock = ENV_MUTEX.lock().unwrap();
    cleanup_env_vars();
    std::env::set_var("TEST_OPENEHR_PASSWORD", "secret_pass");
    std::env::set_var("TEST_COSMOS_KEY", "secret_key");

    let toml_content = r#"database_target = "cosmosdb"
environment = "development"

[application]

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

    use secrecy::ExposeSecret;
    assert_eq!(
        config
            .openehr
            .password
            .as_ref()
            .map(|s| s.expose_secret().as_ref()),
        Some("secret_pass")
    );
    let cosmosdb = config
        .cosmosdb
        .as_ref()
        .expect("CosmosDB config should be present");
    assert_eq!(cosmosdb.key.expose_secret(), "secret_key");

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

    let toml_content = r#"database_target = "cosmosdb"
environment = "development"

[application]
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

    let toml_content = r#"database_target = "cosmosdb"
environment = "development"

[application]
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

#[test]
fn test_env_override_database_target_switch() {
    let _lock = ENV_MUTEX.lock().unwrap();
    cleanup_env_vars();
    std::env::set_var("ATLAS_DATABASE_TARGET", "postgresql");
    std::env::set_var(
        "ATLAS_POSTGRESQL_CONNECTION_STRING",
        "postgresql://localhost/test",
    );

    let toml_content = r#"database_target = "cosmosdb"
environment = "development"

[application]

[openehr]
base_url = "https://ehrbase.example.com"
username = "user"
password = "pass"

[openehr.query]
template_ids = ["template1"]

[export]
mode = "incremental"

[cosmosdb]
endpoint = "https://test.documents.azure.com:443/"
key = "test-key"
database_name = "test_db"

[postgresql]
connection_string = "postgresql://localhost/original"

[state]
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(toml_content.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let config = load_config(temp_file.path()).expect("Failed to load config");

    // Verify database target was overridden
    assert_eq!(
        config.database_target,
        atlas::config::schema::DatabaseTarget::PostgreSQL
    );

    cleanup_env_vars();
}

#[test]
fn test_env_override_comprehensive_application_config() {
    let _lock = ENV_MUTEX.lock().unwrap();
    cleanup_env_vars();
    std::env::set_var("ATLAS_APPLICATION_NAME", "atlas-prod");
    std::env::set_var("ATLAS_APPLICATION_VERSION", "3.5.0");
    std::env::set_var("ATLAS_APPLICATION_LOG_LEVEL", "warn");
    std::env::set_var("ATLAS_APPLICATION_DRY_RUN", "true");

    let toml_content = r#"database_target = "cosmosdb"
environment = "development"

[application]
log_level = "info"
dry_run = false

[openehr]
base_url = "https://ehrbase.example.com"
username = "user"
password = "pass"

[openehr.query]
template_ids = ["template1"]

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

    assert_eq!(config.application.log_level, "warn");
    assert!(config.application.dry_run);

    cleanup_env_vars();
}

#[test]
fn test_env_override_openehr_connection_settings() {
    let _lock = ENV_MUTEX.lock().unwrap();
    cleanup_env_vars();
    std::env::set_var("ATLAS_OPENEHR_BASE_URL", "https://prod-ehrbase.com");
    std::env::set_var("ATLAS_OPENEHR_USERNAME", "prod_user");
    std::env::set_var("ATLAS_OPENEHR_PASSWORD", "prod_password");
    std::env::set_var("ATLAS_OPENEHR_TIMEOUT_SECONDS", "180");
    std::env::set_var("ATLAS_OPENEHR_TLS_VERIFY_CERTIFICATES", "false");

    let toml_content = r#"database_target = "cosmosdb"
environment = "development"

[application]

[openehr]
base_url = "https://ehrbase.example.com"
username = "user"
password = "pass"
timeout_seconds = 60
tls_verify_certificates = true

[openehr.query]
template_ids = ["template1"]

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

    assert_eq!(config.openehr.base_url, "https://prod-ehrbase.com");
    assert_eq!(config.openehr.username, Some("prod_user".to_string()));
    assert_eq!(config.openehr.timeout_seconds, 180);
    assert!(!config.openehr.tls_verify_certificates);

    cleanup_env_vars();
}

#[test]
fn test_env_override_query_arrays_json_format() {
    let _lock = ENV_MUTEX.lock().unwrap();
    cleanup_env_vars();
    std::env::set_var(
        "ATLAS_OPENEHR_QUERY_TEMPLATE_IDS",
        r#"["IDCR - Vital Signs.v1","IDCR - Lab Report.v1","IDCR - Procedures.v1"]"#,
    );
    std::env::set_var(
        "ATLAS_OPENEHR_QUERY_EHR_IDS",
        r#"["ehr-001","ehr-002","ehr-003"]"#,
    );

    let toml_content = r#"database_target = "cosmosdb"
environment = "development"

[application]

[openehr]
base_url = "https://ehrbase.example.com"
username = "user"
password = "pass"

[openehr.query]
template_ids = ["template1"]

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

    assert_eq!(
        config.openehr.query.template_ids,
        vec![
            "IDCR - Vital Signs.v1",
            "IDCR - Lab Report.v1",
            "IDCR - Procedures.v1"
        ]
    );
    assert_eq!(
        config.openehr.query.ehr_ids,
        vec![
            "ehr-001".to_string(),
            "ehr-002".to_string(),
            "ehr-003".to_string()
        ]
    );

    cleanup_env_vars();
}

#[test]
fn test_env_override_query_arrays_csv_format() {
    let _lock = ENV_MUTEX.lock().unwrap();
    cleanup_env_vars();
    std::env::set_var(
        "ATLAS_OPENEHR_QUERY_TEMPLATE_IDS",
        "IDCR - Vital Signs.v1,IDCR - Lab Report.v1",
    );
    std::env::set_var("ATLAS_OPENEHR_QUERY_EHR_IDS", "ehr-123,ehr-456,ehr-789");

    let toml_content = r#"database_target = "cosmosdb"
environment = "development"

[application]

[openehr]
base_url = "https://ehrbase.example.com"
username = "user"
password = "pass"

[openehr.query]
template_ids = ["template1"]

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

    assert_eq!(
        config.openehr.query.template_ids,
        vec!["IDCR - Vital Signs.v1", "IDCR - Lab Report.v1"]
    );
    assert_eq!(
        config.openehr.query.ehr_ids,
        vec![
            "ehr-123".to_string(),
            "ehr-456".to_string(),
            "ehr-789".to_string()
        ]
    );

    cleanup_env_vars();
}

#[test]
fn test_env_override_export_retry_backoff_array() {
    let _lock = ENV_MUTEX.lock().unwrap();
    cleanup_env_vars();
    std::env::set_var("ATLAS_EXPORT_RETRY_BACKOFF_MS", "500,1000,2000,4000,8000");
    std::env::set_var("ATLAS_EXPORT_SHUTDOWN_TIMEOUT_SECS", "120");
    std::env::set_var("ATLAS_EXPORT_DRY_RUN", "true");

    let toml_content = r#"database_target = "cosmosdb"
environment = "development"

[application]

[openehr]
base_url = "https://ehrbase.example.com"
username = "user"
password = "pass"

[openehr.query]
template_ids = ["template1"]

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

    assert_eq!(
        config.export.retry_backoff_ms,
        vec![500, 1000, 2000, 4000, 8000]
    );
    assert_eq!(config.export.shutdown_timeout_secs, 120);
    assert!(config.export.dry_run);

    cleanup_env_vars();
}

#[test]
fn test_env_override_postgresql_all_fields() {
    let _lock = ENV_MUTEX.lock().unwrap();
    cleanup_env_vars();
    std::env::set_var("ATLAS_DATABASE_TARGET", "postgresql");
    std::env::set_var(
        "ATLAS_POSTGRESQL_CONNECTION_STRING",
        "postgresql://prod:secret@prod-db:5432/openehr?sslmode=require",
    );
    std::env::set_var("ATLAS_POSTGRESQL_MAX_CONNECTIONS", "50");
    std::env::set_var("ATLAS_POSTGRESQL_CONNECTION_TIMEOUT_SECONDS", "90");
    std::env::set_var("ATLAS_POSTGRESQL_STATEMENT_TIMEOUT_SECONDS", "180");
    std::env::set_var("ATLAS_POSTGRESQL_SSL_MODE", "verify-full");

    let toml_content = r#"database_target = "postgresql"

[application]

[openehr]
base_url = "https://ehrbase.example.com"
username = "user"
password = "pass"

[openehr.query]
template_ids = ["template1"]

[export]
mode = "incremental"

[postgresql]
connection_string = "postgresql://localhost/test"
max_connections = 10
connection_timeout_seconds = 30
statement_timeout_seconds = 60
ssl_mode = "prefer"

[state]
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(toml_content.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let config = load_config(temp_file.path()).expect("Failed to load config");

    let pg_config = config
        .postgresql
        .as_ref()
        .expect("PostgreSQL config missing");
    assert_eq!(pg_config.max_connections, 50);
    assert_eq!(pg_config.connection_timeout_seconds, 90);
    assert_eq!(pg_config.statement_timeout_seconds, 180);
    assert_eq!(pg_config.ssl_mode, "verify-full".to_string());

    cleanup_env_vars();
}
