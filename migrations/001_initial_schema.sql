-- Atlas PostgreSQL Schema
-- Version: 1.0.0
-- Description: Initial schema for storing OpenEHR compositions and watermarks

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ============================================================================
-- Compositions Table
-- ============================================================================
-- Stores OpenEHR compositions in both preserved and flattened formats
-- Uses JSONB for flexible schema and efficient querying

CREATE TABLE IF NOT EXISTS compositions (
    -- Primary key: composition UID
    id TEXT PRIMARY KEY,
    
    -- EHR identifier (indexed for queries)
    ehr_id TEXT NOT NULL,
    
    -- Composition UID (same as id, kept for consistency with CosmosDB model)
    composition_uid TEXT NOT NULL,
    
    -- Template identifier (indexed for queries)
    template_id TEXT NOT NULL,
    
    -- Timestamp when composition was committed in OpenEHR
    time_committed TIMESTAMPTZ NOT NULL,
    
    -- Composition content in JSONB format
    -- For preserved mode: stores the exact FLAT JSON structure
    -- For flattened mode: stores the flattened key-value pairs
    content JSONB NOT NULL,
    
    -- Export mode: 'preserve' or 'flatten'
    export_mode TEXT NOT NULL,
    
    -- Atlas metadata
    exported_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    atlas_version TEXT NOT NULL,
    checksum TEXT,
    
    -- Constraints
    CONSTRAINT valid_export_mode CHECK (export_mode IN ('preserve', 'flatten'))
);

-- ============================================================================
-- Indexes for Compositions
-- ============================================================================

-- Index on ehr_id for filtering by patient
CREATE INDEX IF NOT EXISTS idx_compositions_ehr_id 
    ON compositions(ehr_id);

-- Index on template_id for filtering by template
CREATE INDEX IF NOT EXISTS idx_compositions_template_id 
    ON compositions(template_id);

-- Index on time_committed for temporal queries
CREATE INDEX IF NOT EXISTS idx_compositions_time_committed 
    ON compositions(time_committed);

-- Composite index for incremental sync queries
CREATE INDEX IF NOT EXISTS idx_compositions_ehr_template 
    ON compositions(ehr_id, template_id, time_committed);

-- GIN index on JSONB content for efficient JSON queries
CREATE INDEX IF NOT EXISTS idx_compositions_content 
    ON compositions USING GIN (content);

-- ============================================================================
-- Watermarks Table
-- ============================================================================
-- Stores state for incremental exports
-- Tracks the last exported composition per {template_id, ehr_id} combination

CREATE TABLE IF NOT EXISTS watermarks (
    -- Primary key: generated from template_id and ehr_id
    id TEXT PRIMARY KEY,
    
    -- Template identifier
    template_id TEXT NOT NULL,
    
    -- EHR identifier
    ehr_id TEXT NOT NULL,
    
    -- Timestamp of the last exported composition
    last_exported_timestamp TIMESTAMPTZ NOT NULL,
    
    -- UID of the last exported composition
    last_exported_composition_uid TEXT,
    
    -- Count of compositions exported for this {template_id, ehr_id}
    compositions_exported_count BIGINT NOT NULL DEFAULT 0,
    
    -- Timestamp when the export started
    last_export_started_at TIMESTAMPTZ NOT NULL,
    
    -- Timestamp when the export completed (NULL if in progress)
    last_export_completed_at TIMESTAMPTZ,
    
    -- Export status: 'in_progress', 'completed', 'failed', 'not_started'
    last_export_status TEXT NOT NULL,

    -- Constraints
    CONSTRAINT valid_export_status CHECK (
        last_export_status IN ('in_progress', 'completed', 'failed', 'not_started')
    ),
    CONSTRAINT unique_template_ehr UNIQUE (template_id, ehr_id)
);

-- ============================================================================
-- Indexes for Watermarks
-- ============================================================================

-- Index on template_id for queries by template
CREATE INDEX IF NOT EXISTS idx_watermarks_template_id 
    ON watermarks(template_id);

-- Index on ehr_id for queries by patient
CREATE INDEX IF NOT EXISTS idx_watermarks_ehr_id 
    ON watermarks(ehr_id);

-- Index on last_export_status for monitoring
CREATE INDEX IF NOT EXISTS idx_watermarks_status 
    ON watermarks(last_export_status);

-- ============================================================================
-- Comments
-- ============================================================================

COMMENT ON TABLE compositions IS 
    'Stores OpenEHR compositions exported from EHRBase in JSONB format';

COMMENT ON COLUMN compositions.id IS 
    'Composition UID (primary key)';

COMMENT ON COLUMN compositions.content IS 
    'Composition content in JSONB - preserved or flattened based on export_mode';

COMMENT ON TABLE watermarks IS 
    'Tracks incremental export state per {template_id, ehr_id} combination';

COMMENT ON COLUMN watermarks.id IS 
    'Generated ID: {template_id}::{ehr_id}';

-- ============================================================================
-- Sample Queries (for reference)
-- ============================================================================

-- Query compositions for a specific EHR and template
-- SELECT * FROM compositions 
-- WHERE ehr_id = '7d44b88c-4199-4bad-97dc-d78268e01398' 
--   AND template_id = 'IDCR - Vital Signs.v1'
-- ORDER BY time_committed DESC;

-- Query compositions with JSON path filtering (preserved mode)
-- SELECT id, ehr_id, content->'ctx/language' as language
-- FROM compositions
-- WHERE template_id = 'IDCR - Lab Report.v1'
--   AND content->>'ctx/language' = 'en';

-- Query compositions with JSON field filtering (flattened mode)
-- SELECT id, ehr_id, content
-- FROM compositions
-- WHERE template_id = 'IDCR - Vital Signs.v1'
--   AND content->>'vital_signs/body_temperature/any_event/temperature|magnitude' > '38';

-- Get watermark for incremental sync
-- SELECT * FROM watermarks
-- WHERE template_id = 'IDCR - Vital Signs.v1'
--   AND ehr_id = '7d44b88c-4199-4bad-97dc-d78268e01398';

-- Get all in-progress exports
-- SELECT * FROM watermarks
-- WHERE last_export_status = 'in_progress';

