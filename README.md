# Atlas

[![Build Status](https://img.shields.io/github/actions/workflow/status/erikhoward/atlas/ci.yml?branch=main)](https://github.com/erikhoward/atlas/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Documentation](https://img.shields.io/badge/docs-latest-brightgreen.svg)](docs/)

**Atlas** is a high-performance, open-source ETL tool built in Rust that bridges OpenEHR clinical data repositories with Microsoft Azure analytics platforms. It enables healthcare organizations to seamlessly export OpenEHR compositions to Azure Cosmos DB for advanced analytics, machine learning, and research.

## 🎯 Overview

Atlas solves the challenge of making OpenEHR clinical data accessible for modern analytics workflows. By exporting compositions from EHRBase servers to Azure Cosmos DB, Atlas enables:

- **Clinical Research**: Query patient data using familiar SQL instead of AQL
- **Machine Learning**: Build ML models on flattened, analytics-ready data
- **Operational Analytics**: Power dashboards and reports with Azure-native tools
- **Regulatory Reporting**: Maintain audit trails with data verification
- **Data Integration**: Connect OpenEHR data to Azure Synapse, Databricks, and Power BI

## ✨ Key Features

### Core Capabilities

- **🚀 High Performance**: Built with Rust for async/concurrent processing
  - Batch processing with configurable sizes (100-5000 compositions)
  - Parallel EHR processing (1-100 concurrent EHRs)
  - Throughput: 1000-2000 compositions/minute

- **🔄 Incremental Sync**: Smart state management with watermarks
  - Track last export per {template_id, ehr_id} combination
  - Export only new/changed data since last run
  - Automatic checkpoint and resume from failures

- **🎨 Flexible Transformation**: Multiple composition formats
  - **Preserve Mode**: Maintain exact FLAT JSON structure from OpenEHR
  - **Flatten Mode**: Convert nested paths to flat field names for ML/analytics

- **⚙️ Easy Configuration**: TOML-based with environment variable support
  - Simple, human-readable configuration files
  - Secure credential management with env vars
  - Comprehensive validation and error messages

- **🛡️ Reliable & Resilient**: Production-ready error handling
  - Automatic retry with exponential backoff
  - Partial batch failure handling
  - Duplicate detection and skipping
  - Optional SHA-256 checksum verification

- **📊 Azure-Native**: Deep integration with Azure services
  - Azure Cosmos DB Core (SQL) API
  - Azure Log Analytics integration (Logs Ingestion API)
  - Kubernetes/AKS deployment support

### Technical Highlights

- **Vendor Abstraction**: Trait-based design supports multiple OpenEHR vendors (EHRBase, Better Platform, Ocean Health)
- **Type Safety**: Strongly-typed domain models with Rust's type system
- **Observability**: Structured logging with tracing, Azure integration
- **Security**: TLS 1.2+, credential management, least-privilege access
- **Compliance**: HIPAA-ready, audit logging, data verification

## 🚀 Quick Start

### Prerequisites

- **Rust 1.70+** (for building from source)
- **OpenEHR Server**: EHRBase 0.30+ with REST API v1.1.x
- **Azure Cosmos DB**: Core (SQL) API account with database created
- **Network Access**: Outbound HTTPS to OpenEHR server and Azure

### Installation

#### Option 1: Pre-built Binary (Recommended)

```bash
# Download latest release
wget https://github.com/erikhoward/atlas/releases/download/v1.0.0/atlas-linux-x86_64.tar.gz

# Extract and install
tar -xzf atlas-linux-x86_64.tar.gz
sudo mv atlas /usr/local/bin/
sudo chmod +x /usr/local/bin/atlas

# Verify installation
atlas --version
```

#### Option 2: Build from Source

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone the repository
git clone https://github.com/erikhoward/atlas.git
cd atlas

# Build release binary
cargo build --release

# Install binary
sudo cp target/release/atlas /usr/local/bin/

# Verify installation
atlas --version
```

#### Option 3: Docker

```bash
# Pull Docker image
docker pull erikhoward/atlas:latest

# Run Atlas
docker run --rm -v $(pwd)/atlas.toml:/app/atlas.toml erikhoward/atlas:latest export
```

### Configuration

```bash
# Generate sample configuration with examples
atlas init --with-examples --output atlas.toml

# Edit configuration for your environment
vi atlas.toml

# Set environment variables for credentials
export ATLAS_OPENEHR_USERNAME="your-openehr-username"
export ATLAS_OPENEHR_PASSWORD="your-openehr-password"
export ATLAS_COSMOSDB_KEY="your-cosmos-db-key"

# Validate configuration
atlas validate-config -c atlas.toml
```

**Minimal Configuration Example**:

```toml
[openehr]
base_url = "https://your-ehrbase-server.com/ehrbase"
username = "${ATLAS_OPENEHR_USERNAME}"
password = "${ATLAS_OPENEHR_PASSWORD}"
tls_verify = true  # Set to false for self-signed certificates in dev

[openehr.query]
template_ids = ["IDCR - Vital Signs.v1"]

[cosmosdb]
endpoint = "https://your-account.documents.azure.com:443/"
key = "${ATLAS_COSMOSDB_KEY}"
database_name = "openehr_data"

[export]
mode = "incremental"
cosmos_composition_format = "preserve"
```

### Basic Usage

```bash
# Run export
atlas export -c atlas.toml

# Dry run to preview (no data written)
atlas export -c atlas.toml --dry-run

# Check export status and watermarks
atlas status -c atlas.toml

# Override configuration options
atlas export -c atlas.toml --mode full --template-id "Your Template.v1"
```

### Example Use Cases

See the [`examples/`](examples/) directory for complete configurations:

- **[Clinical Research](examples/research-export.toml)**: Full export with data verification
- **[Daily Sync](examples/incremental-sync.toml)**: Incremental sync for production
- **[ML Features](examples/ml-features.toml)**: Flattened data for machine learning

## 📖 Documentation

### User Documentation

- **[User Guide](docs/user-guide.md)** - Complete usage instructions, troubleshooting, and best practices
- **[Configuration Guide](docs/configuration.md)** - Detailed configuration reference with all options
- **[Example Configurations](examples/)** - Ready-to-use configs for common scenarios

### Technical Documentation

- **[Architecture Documentation](docs/architecture.md)** - System design, components, and data flow
- **[Developer Guide](docs/developer-guide.md)** - Development setup and contribution guidelines

### Deployment Guides

- **[Standalone Deployment](docs/deployment/standalone.md)** - Binary deployment on Linux/macOS/Windows
- **[Docker Deployment](docs/deployment/docker.md)** - Containerized deployment
- **[Kubernetes Deployment](docs/deployment/kubernetes.md)** - AKS and Kubernetes deployment

## 🏗️ Architecture

Atlas follows a layered architecture with clear separation of concerns:

```text
┌─────────────────────────────────────────────────────────────┐
│                        Atlas CLI                            │
│                     (Rust Binary)                           │
└──────────────┬──────────────────────────┬───────────────────┘
               │                          │
               │ REST API v1.1            │ Azure SDK
               │                          │
               ▼                          ▼
┌──────────────────────────┐   ┌──────────────────────────────┐
│   OpenEHR Server         │   │   Azure Cosmos DB            │
│   (EHRBase 0.30+)        │   │   Core (SQL) API             │
│                          │   │                              │
│  ┌────────────────────┐  │   │  ┌────────────────────────┐  │
│  │  Compositions      │  │   │  │  Control Container     │  │
│  │  (FLAT JSON)       │  │   │  │  - Watermarks          │  │
│  └────────────────────┘  │   │  │  - Export state        │  │
│                          │   │  └────────────────────────┘  │
└──────────────────────────┘   │                              │
                               │  ┌────────────────────────┐  │
                               │  │  Data Containers       │  │
                               │  │  - One per template    │  │
                               │  │  - Partitioned by EHR  │  │
                               │  └────────────────────────┘  │
                               └──────────────────────────────┘
```

**Key Components**:

- **CLI Layer**: Command-line interface with clap
- **Core Layer**: Business logic (export, transform, state, verification)
- **Adapter Layer**: External integrations (OpenEHR, Cosmos DB)
- **Domain Layer**: Core types and models

See [Architecture Documentation](docs/architecture.md) for details.

## 🎯 Use Cases

### Clinical Research

Export patient cohorts for research studies while preserving exact data structures for regulatory compliance.

### Machine Learning

Flatten compositions into analytics-ready format for training predictive models on clinical data.

### Operational Analytics

Power real-time dashboards and reports by syncing OpenEHR data to Cosmos DB daily.

### Data Integration

Connect OpenEHR data to Azure Synapse Analytics, Databricks, or Power BI for advanced analytics.

### Regulatory Reporting

Maintain audit trails with SHA-256 checksums and verification for compliance requirements.

## 🔧 Configuration Options

Atlas supports extensive configuration options:

| Category | Options | Description |
|----------|---------|-------------|
| **Export Mode** | `full`, `incremental` | Full export or incremental sync |
| **Format** | `preserve`, `flatten` | Maintain structure or flatten for analytics |
| **Batch Size** | 100-5000 | Compositions per batch |
| **Parallelism** | 1-100 EHRs | Concurrent EHR processing |
| **Verification** | SHA-256 checksums | Optional data integrity checks |
| **Logging** | Local, Azure Log Analytics | Structured logging options |

See [Configuration Guide](docs/configuration.md) for complete reference.

## 📊 Performance

**Typical Performance** (depends on composition size and network):

- **Throughput**: 1000-2000 compositions/minute
- **Memory**: 2-4 GB RAM (configurable with batch size)
- **Cosmos DB**: ~10 RU per composition write

**Example Scenarios**:

- **Daily Sync**: 1,000 compositions in ~1-2 minutes
- **Research Export**: 50,000 compositions in ~50-100 minutes
- **ML Dataset**: 500,000 compositions in ~4-8 hours

## 🔒 Security

- **TLS 1.2+**: All connections encrypted in transit
- **Credential Management**: Environment variables, never hardcoded
- **Least Privilege**: Read-only OpenEHR access recommended
- **Azure RBAC**: Integrate with Azure role-based access control
- **Audit Logging**: All operations logged with timestamps
- **PHI/PII Protection**: Sanitized logging, compliance-ready

## 🤝 Contributing

We welcome contributions! Here's how to get started:

1. **Fork the repository**
2. **Create a feature branch**: `git checkout -b feature/my-feature`
3. **Make your changes** following the [Developer Guide](docs/developer-guide.md)
4. **Run tests**: `cargo test`
5. **Run linter**: `cargo clippy --all-targets -- -D warnings`
6. **Format code**: `cargo fmt`
7. **Commit changes**: `git commit -m "feat: add new feature"`
8. **Push to branch**: `git push origin feature/my-feature`
9. **Open a Pull Request**

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines.

### Development Setup

```bash
# Clone repository
git clone https://github.com/erikhoward/atlas.git
cd atlas

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install development tools
rustup component add clippy rustfmt

# Build and test
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt
```

## 📝 License

This project is licensed under the **MIT License** - see the [LICENSE](LICENSE) file for details.

## 🆘 Support

### Documentation

- [User Guide](docs/user-guide.md) - Usage instructions and troubleshooting
- [FAQ](docs/user-guide.md#faq) - Frequently asked questions

### Community

- **GitHub Issues**: [Report bugs or request features](https://github.com/erikhoward/atlas/issues)
- **Discussions**: [Ask questions and share ideas](https://github.com/erikhoward/atlas/discussions)

### Commercial Support

For enterprise support, training, or custom development, contact: erikhoward@pm.me

## 🙏 Acknowledgments

Atlas is built with these excellent open-source projects:

- [Rust](https://www.rust-lang.org/) - Systems programming language
- [Tokio](https://tokio.rs/) - Async runtime
- [Clap](https://clap.rs/) - Command-line argument parsing
- [Serde](https://serde.rs/) - Serialization framework
- [Tracing](https://tracing.rs/) - Structured logging
- [Azure SDK for Rust](https://github.com/Azure/azure-sdk-for-rust) - Azure integration

## 🗺️ Roadmap

### Current Version (v1.0)

- ✅ EHRBase vendor support
- ✅ Azure Cosmos DB integration
- ✅ Incremental sync with watermarks
- ✅ Preserve and flatten modes
- ✅ CLI interface
- ✅ Docker and Kubernetes deployment

### Future Enhancements

- 🔄 Additional OpenEHR vendors (Better Platform, Ocean Health)
- 🔄 OAuth 2.0 / OpenID Connect authentication
- 🔄 Azure Data Factory integration
- 🔄 Prometheus metrics export
- 🔄 GraphQL API for querying exported data
- 🔄 Web UI for configuration and monitoring

## 📚 Related Projects

- [EHRBase](https://ehrbase.org/) - Open-source OpenEHR server
- [Azure Cosmos DB](https://azure.microsoft.com/en-us/services/cosmos-db/) - Globally distributed database
- [OpenEHR](https://www.openehr.org/) - Open standard for health data

---

**Made with ❤️ by the Atlas Team**

If you find Atlas useful, please consider giving it a ⭐ on GitHub!
