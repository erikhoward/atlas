# PostgreSQL Setup Guide for Atlas

This guide explains how to set up and configure PostgreSQL as the database backend for Atlas.

## Prerequisites

- PostgreSQL 14 or later
- Database user with CREATE DATABASE and CREATE TABLE privileges
- Network connectivity from Atlas to PostgreSQL server

## Quick Start

### 1. Create Database and User

```sql
-- Connect to PostgreSQL as superuser
psql -U postgres

-- Create database
CREATE DATABASE openehr_data;

-- Create user
CREATE USER atlas_user WITH PASSWORD 'your_secure_password';

-- Grant privileges
GRANT ALL PRIVILEGES ON DATABASE openehr_data TO atlas_user;

-- Connect to the new database
\c openehr_data

-- Grant schema privileges
GRANT ALL ON SCHEMA public TO atlas_user;
```

### 2. Run Database Migration

Atlas includes a SQL migration script to create the required schema. Run it using:

```bash
psql -U atlas_user -d openehr_data -f migrations/001_initial_schema.sql
```

Or manually execute the SQL:

```sql
-- Create compositions table
CREATE TABLE IF NOT EXISTS compositions (
    id UUID PRIMARY KEY,
    ehr_id UUID NOT NULL,
    template_id VARCHAR(255) NOT NULL,
    archetype_node_id VARCHAR(255) NOT NULL,
    time_committed TIMESTAMPTZ NOT NULL,
    content JSONB NOT NULL,
    export_mode VARCHAR(50) NOT NULL,
    exported_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(id, ehr_id)
);

-- Create watermarks table
CREATE TABLE IF NOT EXISTS watermarks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    template_id VARCHAR(255) NOT NULL,
    ehr_id UUID NOT NULL,
    last_composition_id UUID,
    last_time_committed TIMESTAMPTZ,
    last_export_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    export_status VARCHAR(50) NOT NULL DEFAULT 'in_progress',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(template_id, ehr_id),
    CONSTRAINT valid_export_status CHECK (export_status IN ('not_started', 'in_progress', 'completed', 'failed'))
);

-- Create indexes for compositions
CREATE INDEX IF NOT EXISTS idx_compositions_ehr_id ON compositions(ehr_id);
CREATE INDEX IF NOT EXISTS idx_compositions_template_id ON compositions(template_id);
CREATE INDEX IF NOT EXISTS idx_compositions_time_committed ON compositions(time_committed);
CREATE INDEX IF NOT EXISTS idx_compositions_composite ON compositions(ehr_id, template_id, time_committed);
CREATE INDEX IF NOT EXISTS idx_compositions_content_gin ON compositions USING GIN (content);

-- Create indexes for watermarks
CREATE INDEX IF NOT EXISTS idx_watermarks_template_id ON watermarks(template_id);
CREATE INDEX IF NOT EXISTS idx_watermarks_ehr_id ON watermarks(ehr_id);
CREATE INDEX IF NOT EXISTS idx_watermarks_composite ON watermarks(template_id, ehr_id);
CREATE INDEX IF NOT EXISTS idx_watermarks_status ON watermarks(export_status);
```

### 3. Configure Atlas

Create or update your `atlas.toml` configuration file:

```toml
[export]
database_target = "postgresql"

[postgresql]
connection_string = "postgresql://atlas_user:your_secure_password@localhost:5432/openehr_data?sslmode=require"
max_connections = 20
connection_timeout_seconds = 30
statement_timeout_seconds = 60
ssl_mode = "require"
```

See `examples/atlas.postgresql.example.toml` for a complete configuration example.

## Configuration Options

### Connection String Format

The PostgreSQL connection string follows the standard format:

```
postgresql://[user[:password]@][host][:port][/dbname][?param1=value1&...]
```

**Examples:**

```toml
# Local development (no SSL)
connection_string = "postgresql://atlas_user:password@localhost:5432/openehr_data"

# Production with SSL
connection_string = "postgresql://atlas_user:password@db.example.com:5432/openehr_data?sslmode=require"

# Using environment variable for password
connection_string = "postgresql://atlas_user:${ATLAS_PG_PASSWORD}@db.example.com:5432/openehr_data?sslmode=require"

# Azure Database for PostgreSQL
connection_string = "postgresql://atlas_user@myserver:password@myserver.postgres.database.azure.com:5432/openehr_data?sslmode=require"
```

### Connection Pool Settings

| Setting | Description | Default | Recommended |
|---------|-------------|---------|-------------|
| `max_connections` | Maximum connections in pool | 20 | 10-50 depending on workload |
| `connection_timeout_seconds` | Timeout for acquiring connection | 30 | 30-60 |
| `statement_timeout_seconds` | Timeout for query execution | 60 | 60-300 |

### SSL/TLS Modes

| Mode | Description | Use Case |
|------|-------------|----------|
| `disable` | No SSL | Local development only |
| `allow` | Try SSL, fallback to non-SSL | Not recommended |
| `prefer` | Prefer SSL, fallback to non-SSL | Not recommended |
| `require` | Require SSL (no certificate verification) | Production (minimum) |
| `verify-ca` | Require SSL + verify CA | Production (recommended) |
| `verify-full` | Require SSL + verify CA + hostname | Production (most secure) |

**Production Recommendation:** Use `require` or higher.

## Performance Tuning

### PostgreSQL Server Configuration

For optimal performance with Atlas, consider these PostgreSQL settings:

```ini
# postgresql.conf

# Memory settings
shared_buffers = 4GB                    # 25% of system RAM
effective_cache_size = 12GB             # 75% of system RAM
work_mem = 64MB                         # For sorting/hashing
maintenance_work_mem = 1GB              # For VACUUM, CREATE INDEX

# Connection settings
max_connections = 100                   # Adjust based on Atlas instances

# Write-ahead log
wal_buffers = 16MB
checkpoint_completion_target = 0.9

# Query planner
random_page_cost = 1.1                  # For SSD storage
effective_io_concurrency = 200          # For SSD storage

# Parallel query
max_parallel_workers_per_gather = 4
max_parallel_workers = 8
```

### Indexing Strategy

The migration script creates essential indexes. Monitor query performance and add additional indexes as needed:

```sql
-- Example: Index on specific JSONB fields if frequently queried
CREATE INDEX idx_compositions_content_specific 
ON compositions ((content->>'field_name'));

-- Example: Partial index for recent data
CREATE INDEX idx_compositions_recent 
ON compositions(time_committed) 
WHERE time_committed > NOW() - INTERVAL '30 days';
```

### Maintenance

Regular maintenance is important for performance:

```sql
-- Analyze tables (updates statistics)
ANALYZE compositions;
ANALYZE watermarks;

-- Vacuum (reclaim storage)
VACUUM ANALYZE compositions;
VACUUM ANALYZE watermarks;
```

Consider setting up automatic vacuuming:

```ini
# postgresql.conf
autovacuum = on
autovacuum_max_workers = 3
autovacuum_naptime = 1min
```

## Monitoring

### Check Connection Pool Usage

Monitor Atlas logs for connection pool metrics:

```bash
grep "connection pool" /var/log/atlas/atlas.log
```

### Query Performance

Monitor slow queries:

```sql
-- Enable slow query logging in postgresql.conf
log_min_duration_statement = 1000  # Log queries > 1 second

-- View current queries
SELECT pid, usename, application_name, state, query, query_start
FROM pg_stat_activity
WHERE datname = 'openehr_data'
ORDER BY query_start;
```

### Table Statistics

```sql
-- Table sizes
SELECT 
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;

-- Row counts
SELECT 
    'compositions' AS table_name,
    COUNT(*) AS row_count
FROM compositions
UNION ALL
SELECT 
    'watermarks' AS table_name,
    COUNT(*) AS row_count
FROM watermarks;

-- Index usage
SELECT 
    schemaname,
    tablename,
    indexname,
    idx_scan,
    idx_tup_read,
    idx_tup_fetch
FROM pg_stat_user_indexes
WHERE schemaname = 'public'
ORDER BY idx_scan DESC;
```

## Backup and Recovery

### Backup

```bash
# Full database backup
pg_dump -U atlas_user -d openehr_data -F c -f openehr_data_backup.dump

# Backup specific tables
pg_dump -U atlas_user -d openehr_data -t compositions -t watermarks -F c -f atlas_tables_backup.dump

# Backup to SQL format
pg_dump -U atlas_user -d openehr_data > openehr_data_backup.sql
```

### Restore

```bash
# Restore from custom format
pg_restore -U atlas_user -d openehr_data -c openehr_data_backup.dump

# Restore from SQL format
psql -U atlas_user -d openehr_data < openehr_data_backup.sql
```

## Troubleshooting

### Connection Issues

**Problem:** "connection refused"
```bash
# Check PostgreSQL is running
sudo systemctl status postgresql

# Check PostgreSQL is listening on correct port
sudo netstat -plnt | grep 5432

# Check pg_hba.conf allows connections
sudo cat /etc/postgresql/14/main/pg_hba.conf
```

**Problem:** "password authentication failed"
```bash
# Verify user exists
psql -U postgres -c "\du atlas_user"

# Reset password
psql -U postgres -c "ALTER USER atlas_user WITH PASSWORD 'new_password';"
```

### Migration Issues

**Problem:** "Failed to execute migration: Permission denied" or "must be owner of table"

This occurs when the tables already exist but are owned by a different user (e.g., `postgres` instead of `atlas_user`).

**Solution:**
```bash
# Check current table ownership
psql -U atlas_user -d openehr_data -c "\dt"

# If tables are owned by postgres, change ownership as superuser
docker exec -it local-postgres psql -U postgres -d openehr_data -c \
  "ALTER TABLE compositions OWNER TO atlas_user; \
   ALTER TABLE watermarks OWNER TO atlas_user;"

# Or if not using Docker:
psql -U postgres -d openehr_data -c \
  "ALTER TABLE compositions OWNER TO atlas_user; \
   ALTER TABLE watermarks OWNER TO atlas_user;"
```

**Prevention:**
Always run the initial migration as the `atlas_user` to ensure proper ownership:
```bash
psql -U atlas_user -d openehr_data -f migrations/001_initial_schema.sql
```

### Performance Issues

**Problem:** Slow inserts
- Check connection pool size (increase `max_connections`)
- Monitor `pg_stat_activity` for blocking queries
- Ensure indexes are not over-indexed (too many indexes slow writes)
- Check disk I/O performance

**Problem:** Slow queries
- Run `EXPLAIN ANALYZE` on slow queries
- Check if indexes are being used
- Update table statistics with `ANALYZE`
- Consider increasing `work_mem` for complex queries

### Disk Space Issues

```sql
-- Check database size
SELECT pg_size_pretty(pg_database_size('openehr_data'));

-- Check table sizes
SELECT 
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS total_size,
    pg_size_pretty(pg_relation_size(schemaname||'.'||tablename)) AS table_size,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename) - pg_relation_size(schemaname||'.'||tablename)) AS index_size
FROM pg_tables
WHERE schemaname = 'public';
```

## Security Best Practices

1. **Use strong passwords** - Generate secure passwords for database users
2. **Enable SSL/TLS** - Always use `sslmode=require` or higher in production
3. **Restrict network access** - Configure `pg_hba.conf` to allow only necessary hosts
4. **Use environment variables** - Never hardcode passwords in configuration files
5. **Regular updates** - Keep PostgreSQL updated with security patches
6. **Audit logging** - Enable PostgreSQL audit logging for compliance
7. **Principle of least privilege** - Grant only necessary permissions to atlas_user

## Migration from CosmosDB

If you're migrating from CosmosDB to PostgreSQL:

1. **Export data from CosmosDB** (if needed for historical data)
2. **Set up PostgreSQL** following this guide
3. **Update configuration** to use `database_target = "postgresql"`
4. **Run a test export** with a small dataset
5. **Verify data integrity** by comparing record counts
6. **Switch to full production** once validated

Note: Atlas does not provide automatic data migration tools. The two databases can coexist, but Atlas will only write to the configured target.

## Additional Resources

- [PostgreSQL Official Documentation](https://www.postgresql.org/docs/)
- [PostgreSQL Performance Tuning](https://wiki.postgresql.org/wiki/Performance_Optimization)
- [Azure Database for PostgreSQL](https://docs.microsoft.com/en-us/azure/postgresql/)
- [AWS RDS for PostgreSQL](https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/CHAP_PostgreSQL.html)

