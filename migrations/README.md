# Atlas PostgreSQL Migrations

This directory contains SQL migration scripts for the Atlas PostgreSQL database schema.

## Migration Files

- `001_initial_schema.sql` - Initial schema creation (compositions and watermarks tables)

## Running Migrations

### Fresh Installation

For a fresh installation, run the initial schema migration:

```bash
# Using psql directly
psql -U atlas_user -d openehr_data -f migrations/001_initial_schema.sql

# Using Docker
docker exec -i local-postgres psql -U atlas_user -d openehr_data < migrations/001_initial_schema.sql

# Using PowerShell with Docker
Get-Content migrations/001_initial_schema.sql | docker exec -i local-postgres psql -U atlas_user -d openehr_data
```

### Automatic Migration

Atlas automatically runs migrations when you start the application. The migration is idempotent and safe to run multiple times.

## Schema Version History

### Version 1.0.0 (001_initial_schema.sql)

**Compositions Table:**
- `id` (TEXT) - Primary key, composition UID
- `ehr_id` (TEXT) - EHR identifier
- `composition_uid` (TEXT) - Composition UID (same as id)
- `template_id` (TEXT) - Template identifier
- `time_committed` (TIMESTAMPTZ) - Commit timestamp
- `content` (JSONB) - Composition content
- `export_mode` (TEXT) - 'preserve' or 'flatten'
- `exported_at` (TIMESTAMPTZ) - Export timestamp
- `atlas_version` (TEXT) - Atlas version that exported this
- `checksum` (TEXT) - Optional checksum for verification

**Watermarks Table:**
- `id` (TEXT) - Primary key, format: {template_id}::{ehr_id}
- `template_id` (TEXT) - Template identifier
- `ehr_id` (TEXT) - EHR identifier
- `last_exported_timestamp` (TIMESTAMPTZ) - Last export timestamp
- `last_exported_composition_uid` (TEXT) - Last exported composition UID
- `compositions_exported_count` (BIGINT) - Count of exported compositions
- `last_export_started_at` (TIMESTAMPTZ) - Export start time
- `last_export_completed_at` (TIMESTAMPTZ) - Export completion time (NULL if in progress)
- `last_export_status` (TEXT) - 'in_progress', 'completed', 'failed', or 'not_started'

## Troubleshooting

### Schema Mismatch After Refactor

If you've refactored the code and the database schema has changed, you may encounter errors like:

```
column "last_export_status" does not exist
```

This happens when the existing tables have an old schema. You have two options:

#### Option 1: Drop and Recreate (Development Only)

**⚠️ WARNING: This will delete all data!**

```bash
# Drop existing tables
docker exec -it local-postgres psql -U atlas_user -d openehr_data -c \
  "DROP TABLE IF EXISTS compositions CASCADE; DROP TABLE IF EXISTS watermarks CASCADE;"

# Run migration to recreate with new schema
Get-Content migrations/001_initial_schema.sql | docker exec -i local-postgres psql -U atlas_user -d openehr_data
```

#### Option 2: Create an Upgrade Migration (Production)

For production environments with existing data, create a new migration script that alters the existing tables:

```sql
-- Example: migrations/002_upgrade_schema.sql
BEGIN;

-- Alter compositions table
ALTER TABLE compositions 
  ADD COLUMN IF NOT EXISTS atlas_version TEXT,
  ADD COLUMN IF NOT EXISTS checksum TEXT,
  DROP COLUMN IF EXISTS archetype_node_id;

-- Alter watermarks table
ALTER TABLE watermarks
  RENAME COLUMN export_status TO last_export_status;

-- Add new columns
ALTER TABLE watermarks
  ADD COLUMN IF NOT EXISTS last_exported_timestamp TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS last_exported_composition_uid TEXT,
  ADD COLUMN IF NOT EXISTS compositions_exported_count BIGINT DEFAULT 0,
  ADD COLUMN IF NOT EXISTS last_export_started_at TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS last_export_completed_at TIMESTAMPTZ;

COMMIT;
```

### Permission Errors

If you see "must be owner of table" errors:

```bash
# Change table ownership to atlas_user
docker exec -it local-postgres psql -U postgres -d openehr_data -c \
  "ALTER TABLE compositions OWNER TO atlas_user; \
   ALTER TABLE watermarks OWNER TO atlas_user;"
```

### Verifying Schema

Check the current schema:

```bash
# View table structure
docker exec -it local-postgres psql -U atlas_user -d openehr_data -c "\d compositions"
docker exec -it local-postgres psql -U atlas_user -d openehr_data -c "\d watermarks"

# List all tables
docker exec -it local-postgres psql -U atlas_user -d openehr_data -c "\dt"

# Check table ownership
docker exec -it local-postgres psql -U atlas_user -d openehr_data -c "\dt"
```

## Best Practices

1. **Always backup before migrations** - Use `pg_dump` to backup your database
2. **Test migrations in development first** - Never run untested migrations in production
3. **Use transactions** - Wrap migrations in BEGIN/COMMIT for atomicity
4. **Make migrations idempotent** - Use `IF NOT EXISTS`, `IF EXISTS` clauses
5. **Version control migrations** - Keep all migration scripts in git
6. **Document schema changes** - Update this README with each migration

## Migration Checklist

Before running a migration:

- [ ] Backup the database
- [ ] Review the migration SQL
- [ ] Test in development environment
- [ ] Check for data loss risks
- [ ] Verify table ownership
- [ ] Plan rollback strategy
- [ ] Schedule maintenance window (if needed)
- [ ] Notify stakeholders

After running a migration:

- [ ] Verify schema changes
- [ ] Test application functionality
- [ ] Check for errors in logs
- [ ] Monitor performance
- [ ] Update documentation
- [ ] Commit migration script to git

## Rollback Procedures

If a migration fails or causes issues:

1. **Stop the application** to prevent further writes
2. **Restore from backup** if data corruption occurred
3. **Investigate the error** in PostgreSQL logs
4. **Fix the migration script** and test again
5. **Document the issue** for future reference

## Additional Resources

- [PostgreSQL ALTER TABLE Documentation](https://www.postgresql.org/docs/current/sql-altertable.html)
- [PostgreSQL Migration Best Practices](https://wiki.postgresql.org/wiki/Don%27t_Do_This)
- [Atlas PostgreSQL Setup Guide](../docs/postgresql-setup.md)

