# Atlas Example Configurations

This directory contains example configuration files for common Atlas use cases. Each configuration is optimized for a specific scenario and includes detailed comments explaining the settings.

## Available Examples

### 1. Clinical Research Export (`research-export.toml`)

**Use Case**: Export clinical data for research studies

**Key Features**:
- **Export Mode**: Full export of all compositions within a time range
- **Composition Format**: Preserve (maintains exact FLAT JSON structure)
- **Verification**: Enabled (SHA-256 checksums for data integrity)
- **Batch Size**: 2000 (optimized for throughput)
- **Parallel EHRs**: 8 (moderate parallelism for stability)

**Best For**:
- Clinical research studies requiring exact data preservation
- One-time or periodic exports (weekly/monthly)
- Data quality audits and verification
- Regulatory compliance scenarios

**Expected Performance**:
- Volume: 10,000-100,000 compositions
- Throughput: ~500-1000 compositions/minute
- Duration: 10-100 minutes (depending on volume)

**Quick Start**:
```bash
# Set environment variables
export ATLAS_OPENEHR_PASSWORD="your-password"
export ATLAS_COSMOS_KEY="your-cosmos-key"
export ATLAS_APP_INSIGHTS_KEY="your-insights-key"

# Validate configuration
atlas validate-config -c examples/research-export.toml

# Run export
atlas export -c examples/research-export.toml
```

---

### 2. Incremental Daily Sync (`incremental-sync.toml`)

**Use Case**: Daily synchronization of new/updated compositions

**Key Features**:
- **Export Mode**: Incremental (only new data since last run)
- **Composition Format**: Flatten (easier querying in Cosmos DB)
- **Verification**: Disabled (prioritize speed)
- **Batch Size**: 1000 (balanced for daily sync)
- **Parallel EHRs**: 16 (high parallelism for speed)
- **Checkpointing**: Every 30 seconds (frequent saves)

**Best For**:
- Nightly data synchronization
- Keeping Cosmos DB up-to-date with OpenEHR
- Production environments with regular data updates
- Automated scheduled exports

**Expected Performance**:
- Volume: 100-5,000 new compositions per day
- Throughput: ~1000-2000 compositions/minute
- Duration: 1-5 minutes per run

**Quick Start**:
```bash
# Set environment variables
export ATLAS_OPENEHR_PASSWORD="your-password"
export ATLAS_COSMOS_KEY="your-cosmos-key"

# Initial full export (first run only)
atlas export -c examples/incremental-sync.toml --mode full

# Schedule daily sync (cron example)
# Add to crontab: Run daily at 2 AM
0 2 * * * source /etc/atlas/atlas.env && /usr/local/bin/atlas export -c /etc/atlas/incremental-sync.toml >> /var/log/atlas/cron.log 2>&1
```

**Scheduling Options**:

**Cron (Linux/macOS)**:
```bash
# Edit crontab
crontab -e

# Add entry (daily at 2 AM)
0 2 * * * source /etc/atlas/atlas.env && atlas export -c /etc/atlas/incremental-sync.toml
```

**Systemd Timer (Linux)**:
```bash
# Enable and start timer
sudo systemctl enable atlas.timer
sudo systemctl start atlas.timer
```

**Kubernetes CronJob**:
```bash
# Deploy CronJob
kubectl apply -f manifests/cronjob-incremental-sync.yaml
```

---

### 3. ML Feature Extraction (`ml-features.toml`)

**Use Case**: Export data for machine learning and analytics

**Key Features**:
- **Export Mode**: Full export for comprehensive ML dataset
- **Composition Format**: Flatten (ML-friendly flat structure)
- **Verification**: Disabled (prioritize speed)
- **Batch Size**: 5000 (maximum for high throughput)
- **Parallel EHRs**: 32 (maximum parallelism)
- **Concurrency**: 50 (high Cosmos DB concurrency)

**Best For**:
- Machine learning model training
- Feature engineering and data science
- Analytics and reporting
- Large-scale data exports (100,000+ compositions)

**Expected Performance**:
- Volume: 100,000-500,000 compositions
- Throughput: ~1000-2000 compositions/minute
- Duration: 50 minutes to 8 hours (depending on volume)

**Quick Start**:
```bash
# Set environment variables
export ATLAS_OPENEHR_PASSWORD="your-password"
export ATLAS_COSMOS_KEY="your-cosmos-key"

# Validate configuration
atlas validate-config -c examples/ml-features.toml

# Run export (consider running overnight)
nohup atlas export -c examples/ml-features.toml > ml-export.log 2>&1 &

# Monitor progress
tail -f ml-export.log
```

**ML Pipeline Integration**:

After export, integrate with your ML platform:

**Azure Machine Learning**:
```python
from azureml.core import Workspace, Dataset

# Connect to Cosmos DB
dataset = Dataset.Tabular.from_cosmos_db(
    endpoint=cosmos_endpoint,
    database=database_name,
    container=container_name,
    key=cosmos_key
)

# Use in ML pipeline
df = dataset.to_pandas_dataframe()
```

**Azure Databricks**:
```python
# Read from Cosmos DB
df = spark.read.format("cosmos.oltp") \
    .option("spark.cosmos.accountEndpoint", endpoint) \
    .option("spark.cosmos.accountKey", key) \
    .option("spark.cosmos.database", "ml_features") \
    .option("spark.cosmos.container", "compositions_vital_signs") \
    .load()

# Feature engineering
features = df.select("ehr_id", "vital_signs_blood_pressure_systolic", "vital_signs_heart_rate")
```

---

## Customizing Configurations

### Common Customizations

1. **Change Templates**:
   ```toml
   [openehr.query]
   template_ids = ["Your Template.v1", "Another Template.v1"]
   ```

2. **Filter by Patients**:
   ```toml
   [openehr.query]
   ehr_ids = ["ehr-001", "ehr-002", "ehr-003"]
   ```

3. **Set Time Range**:
   ```toml
   [openehr.query]
   time_range_start = "2024-01-01T00:00:00Z"
   time_range_end = "2024-12-31T23:59:59Z"
   ```

4. **Adjust Performance**:
   ```toml
   [openehr.query]
   batch_size = 2000  # Increase for higher throughput
   parallel_ehrs = 16  # Increase for more parallelism
   
   [cosmosdb]
   max_concurrency = 20  # Increase for higher Cosmos DB throughput
   ```

5. **Enable Verification**:
   ```toml
   [verification]
   enable_verification = true
   ```

### Creating Your Own Configuration

1. **Start with an example**:
   ```bash
   cp examples/incremental-sync.toml my-config.toml
   ```

2. **Edit for your environment**:
   ```bash
   vi my-config.toml
   ```

3. **Validate**:
   ```bash
   atlas validate-config -c my-config.toml
   ```

4. **Test with dry-run**:
   ```bash
   atlas export -c my-config.toml --dry-run
   ```

5. **Run export**:
   ```bash
   atlas export -c my-config.toml
   ```

## Configuration Comparison

| Feature | Research Export | Incremental Sync | ML Features |
|---------|----------------|------------------|-------------|
| **Export Mode** | Full | Incremental | Full |
| **Format** | Preserve | Flatten | Flatten |
| **Verification** | Enabled | Disabled | Disabled |
| **Batch Size** | 2000 | 1000 | 5000 |
| **Parallel EHRs** | 8 | 16 | 32 |
| **Concurrency** | 15 | 20 | 50 |
| **Checkpointing** | 60s | 30s | 60s |
| **Use Case** | Research | Daily Sync | ML/Analytics |
| **Frequency** | Periodic | Daily | One-time |
| **Volume** | 10K-100K | 100-5K/day | 100K-500K |
| **Duration** | 10-100 min | 1-5 min | 50 min-8 hrs |

## Environment Variables

All examples use environment variables for sensitive credentials:

```bash
# Required
export ATLAS_OPENEHR_USERNAME="your-openehr-username"
export ATLAS_OPENEHR_PASSWORD="your-openehr-password"
export ATLAS_COSMOSDB_KEY="your-cosmos-db-key"

# Optional (for Azure logging)
export AZURE_INSTRUMENTATION_KEY="your-application-insights-key"
export AZURE_LOG_ANALYTICS_WORKSPACE_ID="your-workspace-id"
export AZURE_LOG_ANALYTICS_SHARED_KEY="your-shared-key"
```

**Persistent Configuration** (add to `~/.bashrc` or `~/.zshrc`):
```bash
echo 'export ATLAS_OPENEHR_USERNAME="your-username"' >> ~/.bashrc
echo 'export ATLAS_OPENEHR_PASSWORD="your-password"' >> ~/.bashrc
echo 'export ATLAS_COSMOSDB_KEY="your-key"' >> ~/.bashrc
source ~/.bashrc
```

## Troubleshooting

### Configuration Validation Fails

**Problem**: `atlas validate-config` reports errors

**Solution**:
1. Check syntax (TOML format)
2. Verify all required fields are present
3. Check environment variables are set
4. Test connections manually

### Export is Slow

**Problem**: Export takes longer than expected

**Solution**:
1. Increase `batch_size` (try 2000-5000)
2. Increase `parallel_ehrs` (try 16-32)
3. Increase `max_concurrency` (try 20-50)
4. Check network latency
5. Monitor Cosmos DB RU consumption

### Out of Memory

**Problem**: Atlas crashes with OOM error

**Solution**:
1. Reduce `batch_size` (try 500-1000)
2. Reduce `parallel_ehrs` (try 4-8)
3. Increase system memory
4. Monitor memory usage: `ps aux | grep atlas`

### Cosmos DB Throttling

**Problem**: Seeing 429 errors in logs

**Solution**:
1. Increase provisioned RU/s in Cosmos DB
2. Reduce `max_concurrency`
3. Increase `retry_backoff_ms`
4. Monitor RU consumption in Azure Portal

## Additional Resources

- [Configuration Guide](../docs/configuration.md) - Complete configuration reference
- [User Guide](../docs/user-guide.md) - Detailed usage instructions
- [Architecture Documentation](../docs/architecture.md) - System architecture
- [Deployment Guides](../docs/deployment/) - Deployment options

## Support

For issues or questions:
1. Check the [User Guide](../docs/user-guide.md) troubleshooting section
2. Review logs in `/var/log/atlas/`
3. Open an issue on GitHub
4. Contact the Atlas team

---

**Note**: Remember to never commit configuration files with actual credentials to version control. Always use environment variables for sensitive data.

