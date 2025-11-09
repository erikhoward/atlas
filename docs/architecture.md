# Atlas Architecture Documentation

This document describes the system architecture, component design, and data flow of Atlas.

## Table of Contents

- [High-Level Architecture](#high-level-architecture)
- [Component Architecture](#component-architecture)
- [Data Flow](#data-flow)
- [Key Design Patterns](#key-design-patterns)
- [Extension Points](#extension-points)
- [Performance Considerations](#performance-considerations)
- [Security Architecture](#security-architecture)

## High-Level Architecture

Atlas is a command-line ETL tool that extracts OpenEHR compositions from EHRBase servers and loads them into either Azure Cosmos DB or PostgreSQL for analytics and querying.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                            Atlas CLI                                    │
│                         (Rust Binary)                                   │
└──────────────┬──────────────────────────────────────┬───────────────────┘
               │                                      │
               │ REST API v1.1                        │ Database Adapters
               │                                      │
               ▼                                      ▼
┌──────────────────────────┐   ┌──────────────────────────────────────────┐
│   OpenEHR Server         │   │         Database Backends                │
│   (EHRBase 0.30+)        │   │                                          │
│                          │   │  ┌────────────────────────────────────┐  │
│  ┌────────────────────┐  │   │  │  Azure Cosmos DB (NoSQL)           │  │
│  │  Compositions      │  │   │  │  - Control Container (watermarks)  │  │
│  │  (FLAT JSON)       │  │   │  │  - Data Containers (per template)  │  │
│  └────────────────────┘  │   │  │  - Partitioned by /ehr_id          │  │
│                          │   │  └────────────────────────────────────┘  │
└──────────────────────────┘   │                                          │
                               │  ┌────────────────────────────────────┐  │
                               │  │  PostgreSQL 14+ (Relational)       │  │
                               │  │  - atlas_watermarks table          │  │
                               │  │  - compositions_* tables           │  │
                               │  │  - JSONB columns for flexibility   │  │
                               │  └────────────────────────────────────┘  │
                               └──────────────────────────────────────────┘
```

### Key Components

1. **Atlas CLI**: Command-line interface for configuration, validation, and export operations
2. **OpenEHR Server**: Source system containing clinical compositions (EHRBase implementation)
3. **Database Backends**: Target systems for analytics-ready data storage
   - **Azure Cosmos DB**: NoSQL database with control container for state and data containers per template
   - **PostgreSQL**: Relational database with watermarks table and composition tables with JSONB support

## Component Architecture

Atlas follows a layered architecture with clear separation of concerns:

```
┌─────────────────────────────────────────────────────────────┐
│                         CLI Layer                           │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │  export  │  │ validate │  │  status  │  │   init   │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                        Core Layer                           │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────┐  │
│  │  Export          │  │  Transform       │  │  State   │  │
│  │  - Coordinator   │  │  - Preserve      │  │  Manager │  │
│  │  - Batch         │  │  - Flatten       │  │          │  │
│  │  - Summary       │  │                  │  │          │  │
│  └──────────────────┘  └──────────────────┘  └──────────┘  │
│                                                             │
│  ┌──────────────────┐                                      │
│  │  Verification    │                                      │
│  │  - Existence     │                                      │
│  │  - Report        │                                      │
│  └──────────────────┘                                      │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                      Adapter Layer                          │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────┐  │
│  │  OpenEHR         │  │  Cosmos DB       │  │PostgreSQL│  │
│  │  - Vendor Trait  │  │  - Client        │  │ - Client │  │
│  │  - EHRBase Impl  │  │  - Models        │  │ - Models │  │
│  │  - Client        │  │  - Bulk Ops      │  │ - Pool   │  │
│  └──────────────────┘  └──────────────────┘  └──────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                      Domain Layer                           │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │   IDs    │  │  Models  │  │  Errors  │  │  Result  │   │
│  │  - EhrId │  │  - Comp  │  │  - Atlas │  │   Type   │   │
│  │  - UID   │  │  - Ehr   │  │   Error  │  │          │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                   Cross-Cutting Concerns                    │
│  ┌──────────────────┐  ┌──────────────────┐               │
│  │  Configuration   │  │  Logging         │               │
│  │  - TOML Loader   │  │  - Structured    │               │
│  │  - Validation    │  │  - Azure         │               │
│  └──────────────────┘  └──────────────────┘               │
└─────────────────────────────────────────────────────────────┘
```

### Module Descriptions

#### CLI Layer (`src/cli/`)
- **Purpose**: User interface and command handling
- **Commands**:
  - `export`: Execute data export from OpenEHR to database (Cosmos DB or PostgreSQL)
  - `validate-config`: Validate configuration file
  - `status`: Display export status and watermarks
  - `init`: Generate sample configuration files
- **Technology**: `clap` v4 for argument parsing

#### Core Layer (`src/core/`)
- **Purpose**: Business logic and orchestration
- **Export Module** (`export/`):
  - `coordinator.rs`: Orchestrates the entire export process
  - `batch.rs`: Batch processing with configurable sizes
  - `summary.rs`: Export summary and error reporting
- **Transform Module** (`transform/`):
  - `preserve.rs`: Maintains exact FLAT JSON structure
  - `flatten.rs`: Converts nested paths to flat field names
  - `mod.rs`: Strategy pattern for transformation selection
- **State Module** (`state/`):
  - `manager.rs`: Watermark persistence to database
  - `watermark.rs`: High-watermark tracking model
- **Verification Module** (`verification/`):
  - `report.rs`: Verification report generation
  - `verify.rs`: Post-export validation logic

#### Adapter Layer (`src/adapters/`)
- **Purpose**: External system integrations
- **OpenEHR Adapter** (`openehr/`):
  - `vendor/trait.rs`: Vendor abstraction trait
  - `vendor/ehrbase.rs`: EHRBase-specific implementation
  - `client.rs`: HTTP client with retry logic
  - `models.rs`: OpenEHR domain models
- **Cosmos DB Adapter** (`cosmosdb/`):
  - `client.rs`: Cosmos DB connection management
  - `models.rs`: Document models for Cosmos DB
  - Bulk operations with retry and error handling
- **PostgreSQL Adapter** (`postgresql/`):
  - `client.rs`: PostgreSQL connection pool management
  - `models.rs`: Table models and JSONB handling
  - Transaction support and batch operations

#### Domain Layer (`src/domain/`)
- **Purpose**: Core domain types and business rules
- **Types**:
  - `ids.rs`: Strongly-typed identifiers (EhrId, CompositionUid, TemplateId)
  - `composition.rs`: Composition domain model
  - `ehr.rs`: EHR domain model
  - `errors.rs`: Domain-specific error types
- **Pattern**: Type-driven design with newtype pattern for IDs

#### Cross-Cutting Concerns
- **Configuration** (`src/config/`):
  - TOML parsing with `serde`
  - Environment variable substitution
  - Schema validation
- **Logging** (`src/logging/`):
  - Structured logging with `tracing`
  - Azure Log Analytics integration (Logs Ingestion API)

## Data Flow

### Export Process Flow

```
┌─────────────────────────────────────────────────────────────┐
│ 1. Initialization                                           │
│    ├─> Load configuration (TOML + env vars)                 │
│    ├─> Validate configuration                               │
│    ├─> Initialize logging                                   │
│    ├─> Connect to OpenEHR server                            │
│    └─> Connect to database (Cosmos DB or PostgreSQL)        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│ 2. State Loading (Incremental Mode)                         │
│    ├─> Read watermarks from database                        │
│    ├─> Determine last export timestamp per {template, ehr}  │
│    └─> Calculate compositions to export                     │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│ 3. Query OpenEHR (Parallel by EHR)                          │
│    ├─> Get EHR list (if not specified in config)            │
│    ├─> For each EHR (parallel, configurable concurrency):   │
│    │   ├─> Query compositions by template ID                │
│    │   ├─> Filter by time_committed (incremental)           │
│    │   └─> Fetch composition in FLAT format                 │
│    └─> Collect all compositions                             │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│ 4. Transform (Strategy Pattern)                             │
│    ├─> Select transformation mode from config               │
│    ├─> Preserve Mode:                                       │
│    │   ├─> Maintain exact FLAT JSON structure               │
│    │   └─> Add atlas_metadata section                       │
│    └─> Flatten Mode:                                        │
│        ├─> Convert paths to flat field names                │
│        └─> Add atlas_metadata section                       │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│ 5. Load to Database (Batch Processing)                      │
│    ├─> Group compositions into batches (100-5000)           │
│    ├─> For each batch:                                      │
│    │   ├─> Check for duplicates (by composition UID)        │
│    │   ├─> Bulk insert to database                          │
│    │   ├─> Handle partial failures                          │
│    │   └─> Update watermark (checkpoint)                    │
│    └─> Collect batch results                                │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│ 6. Verification (Optional)                                  │
│    ├─> Fetch exported compositions from database            │
│    ├─> Verify document existence                            │
│    └─> Generate verification report                         │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│ 7. Reporting                                                │
│    ├─> Generate export summary                              │
│    ├─> Log statistics (total, success, failed, skipped)     │
│    ├─> Log errors with categorization                       │
│    └─> Return exit code (0=success, 1=partial, 2+=error)    │
└─────────────────────────────────────────────────────────────┘
```

### Incremental Export Logic

```
┌─────────────────────────────────────────────────────────────┐
│ Load Watermark for {template_id, ehr_id}                    │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
                    ┌─────────────────┐
                    │ Watermark Exists?│
                    └─────────────────┘
                       │            │
                  Yes  │            │  No
                       ▼            ▼
        ┌──────────────────┐   ┌──────────────────┐
        │ Incremental Mode │   │   Full Export    │
        │                  │   │                  │
        │ Query since:     │   │ Query all        │
        │ last_exported_   │   │ compositions     │
        │ timestamp        │   │                  │
        └──────────────────┘   └──────────────────┘
                       │            │
                       └────────┬───┘
                                ▼
                    ┌──────────────────────┐
                    │ Fetch Compositions   │
                    │ in FLAT format       │
                    └──────────────────────┘
                                │
                                ▼
                    ┌──────────────────────┐
                    │ Transform & Load     │
                    └──────────────────────┘
                                │
                                ▼
                    ┌──────────────────────┐
                    │ Update Watermark     │
                    │ - timestamp          │
                    │ - composition count  │
                    │ - status             │
                    └──────────────────────┘
```

## Key Design Patterns

### 1. Vendor Abstraction (Strategy Pattern)

The `OpenEhrVendor` trait allows Atlas to support multiple OpenEHR implementations:

```rust
#[async_trait]
pub trait OpenEhrVendor: Send + Sync {
    async fn get_ehr_ids(&self) -> Result<Vec<EhrId>>;
    async fn get_compositions_for_ehr(...) -> Result<Vec<CompositionMetadata>>;
    async fn fetch_composition(...) -> Result<Composition>;
    async fn authenticate(&mut self) -> Result<()>;
}
```

**Benefits**:
- Easy to add new OpenEHR vendors (e.g., Better Platform, Ocean Health)
- Testable with mock implementations
- Vendor-specific optimizations without affecting core logic

### 2. Transformation Strategy Pattern

Transform selection based on configuration:

```rust
pub fn transform_composition(
    composition: &Composition,
    format: &str,
) -> Result<Value> {
    match format {
        "preserve" => preserve::preserve_composition(composition),
        "flatten" => flatten::flatten_composition(composition),
        _ => Err(AtlasError::Configuration(...)),
    }
}
```

### 3. Watermark Pattern (State Management)

High-watermark tracking for incremental exports:

```rust
pub struct Watermark {
    pub template_id: TemplateId,
    pub ehr_id: EhrId,
    pub last_exported_timestamp: DateTime<Utc>,
    pub last_exported_composition_uid: Option<CompositionUid>,
    pub compositions_exported_count: u64,
    pub last_export_status: ExportStatus,
}
```

**Benefits**:
- Resume from failures without re-exporting data
- Track progress per {template, EHR} combination
- Support for partial exports and checkpointing

### 4. Batch Processing Pattern

Configurable batch sizes for optimal throughput:

```rust
pub struct BatchProcessor {
    cosmos_client: Arc<CosmosDbClient>,
    state_manager: Arc<StateManager>,
}

impl BatchProcessor {
    pub async fn process_batch(
        &self,
        compositions: Vec<Composition>,
        config: &BatchConfig,
    ) -> Result<BatchResult> {
        // Transform, insert, checkpoint
    }
}
```

### 5. Error Categorization

Structured error handling with categorization:

```rust
pub enum ExportError {
    OpenEhrConnectionError { ... },
    CosmosDbConnectionError { ... },
    TransformationError { ... },
    DuplicateComposition { ... },
    PartialBatchFailure { ... },
}
```

## Extension Points

### Adding a New OpenEHR Vendor

1. Create a new file: `src/adapters/openehr/vendor/your_vendor.rs`
2. Implement the `OpenEhrVendor` trait
3. Add vendor-specific configuration to `OpenEhrConfig`
4. Update the vendor factory in `src/adapters/openehr/client.rs`
5. Add integration tests

Example:
```rust
pub struct BetterPlatformVendor {
    base_url: String,
    client: reqwest::Client,
    config: OpenEhrConfig,
}

#[async_trait]
impl OpenEhrVendor for BetterPlatformVendor {
    // Implement trait methods
}
```

### Adding a New Transformation Mode

1. Create a new file: `src/core/transform/your_mode.rs`
2. Implement transformation function
3. Update the strategy selector in `src/core/transform/mod.rs`
4. Add configuration option
5. Add tests

### Adding a New Target Database

1. Create a new adapter: `src/adapters/your_db/`
2. Implement client and models
3. Update export coordinator to support multiple targets
4. Add configuration section

## Performance Considerations

### Parallelism

- **EHR-level parallelism**: Process multiple EHRs concurrently (configurable: 1-100)
- **Batch processing**: Group compositions for bulk operations (100-5000 per batch)
- **Async I/O**: Non-blocking operations with Tokio runtime

### Optimization Strategies

1. **Batch Size Tuning**:
   - Smaller batches (100-500): Lower memory, more frequent checkpoints
   - Larger batches (2000-5000): Higher throughput, less overhead

2. **Parallel EHR Processing**:
   - Default: 8 concurrent EHRs
   - High-throughput: 16-32 concurrent EHRs
   - Resource-constrained: 2-4 concurrent EHRs

3. **Cosmos DB Concurrency**:
   - Default: 10 concurrent operations
   - High-throughput: 20-50 concurrent operations
   - Monitor RU consumption and throttling

4. **Checkpointing Interval**:
   - Frequent (10-30s): Better fault tolerance, more overhead
   - Infrequent (60-120s): Less overhead, longer recovery time

### Resource Requirements

- **Memory**: ~100MB base + (batch_size × 50KB per composition)
- **CPU**: Minimal (I/O bound workload)
- **Network**: Depends on composition size and throughput
- **Cosmos DB RUs**: ~10 RU per composition write

## Security Architecture

### Authentication

- **OpenEHR**: Basic Authentication (username/password)
- **Cosmos DB**: Primary/Secondary key authentication
- **PostgreSQL**: Username/password authentication with SSL/TLS
- **Future**: OAuth 2.0 / OpenID Connect support

### Data Protection

- **In Transit**: TLS 1.2+ for all connections
- **At Rest**:
  - Cosmos DB: Encryption managed by Azure
  - PostgreSQL: Database-level encryption (depends on deployment)
- **Credentials**: Environment variable substitution, never hardcoded

### Access Control

- **Principle of Least Privilege**: OpenEHR user should have read-only access
- **Database Access**:
  - Cosmos DB: Use keys with minimum required permissions
  - PostgreSQL: Use dedicated user with INSERT/UPDATE permissions only
- **Logging**: Sanitize PHI/PII from logs

### Compliance Considerations

- **HIPAA**: Ensure BAA with Azure, enable audit logging
- **GDPR**: Support for data deletion (manual process)
- **Audit Trail**: All operations logged with timestamps and user context

---

For more information, see:
- [Configuration Guide](configuration.md) - Detailed configuration options
- [User Guide](user-guide.md) - Usage instructions and examples
- [Developer Guide](developer-guide.md) - Development and contribution guidelines

