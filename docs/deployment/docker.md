# Docker Deployment Guide

This guide covers deploying Atlas as a Docker container.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Building the Docker Image](#building-the-docker-image)
- [Running Atlas in Docker](#running-atlas-in-docker)
- [Docker Compose](#docker-compose)
- [Configuration Management](#configuration-management)
- [Persistent Storage](#persistent-storage)
- [Monitoring and Logging](#monitoring-and-logging)
- [Production Deployment](#production-deployment)
- [Troubleshooting](#troubleshooting)

## Prerequisites

- **Docker**: Version 20.10 or later
- **Docker Compose**: Version 2.0 or later (optional)
- **Access to**: OpenEHR server and Azure Cosmos DB

Install Docker:

**Linux**:
```bash
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh
sudo usermod -aG docker $USER
```

**macOS**:
```bash
brew install --cask docker
```

**Windows**:
Download and install [Docker Desktop](https://www.docker.com/products/docker-desktop)

## Building the Docker Image

### Dockerfile

Create a `Dockerfile` in the project root:

```dockerfile
# Build stage
FROM rust:1.70-slim as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code and migrations (migrations are embedded at compile time via include_str!)
COPY src ./src
COPY migrations ./migrations

# Build release binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1000 atlas

# Create directories
RUN mkdir -p /app /etc/atlas /var/log/atlas && \
    chown -R atlas:atlas /app /etc/atlas /var/log/atlas

# Copy binary from builder
COPY --from=builder /app/target/release/atlas /usr/local/bin/atlas

# Set user
USER atlas

WORKDIR /app

# Default command
ENTRYPOINT ["atlas"]
CMD ["--help"]
```

### Build the Image

```bash
# Build image
docker build -t atlas:latest .

# Build with specific version tag
docker build -t atlas:2.4.0 .

# Verify image
docker images | grep atlas
```

### Multi-platform Build (Optional)

Build for multiple architectures:

```bash
# Create buildx builder
docker buildx create --name atlas-builder --use

# Build for multiple platforms
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t your-registry/atlas:latest \
  --push \
  .
```

## Running Atlas in Docker

### Basic Run

```bash
# Run with default help
docker run --rm atlas:latest

# Run export command
docker run --rm \
  -v $(pwd)/atlas.toml:/etc/atlas/atlas.toml:ro \
  -e ATLAS_OPENEHR_PASSWORD=your-password \
  -e ATLAS_COSMOS_KEY=your-key \
  atlas:latest export -c /etc/atlas/atlas.toml
```

### Run with Volume Mounts

```bash
# Mount configuration and logs
docker run --rm \
  -v $(pwd)/atlas.toml:/etc/atlas/atlas.toml:ro \
  -v $(pwd)/logs:/var/log/atlas \
  -e ATLAS_OPENEHR_PASSWORD=your-password \
  -e ATLAS_COSMOS_KEY=your-key \
  atlas:latest export -c /etc/atlas/atlas.toml
```

### Run with Environment File

Create `.env` file:
```bash
ATLAS_OPENEHR_PASSWORD=your-password
ATLAS_COSMOS_KEY=your-key
```

Run with env file:
```bash
docker run --rm \
  -v $(pwd)/atlas.toml:/etc/atlas/atlas.toml:ro \
  --env-file .env \
  atlas:latest export -c /etc/atlas/atlas.toml
```

### Interactive Mode

```bash
# Run interactive shell
docker run -it --rm \
  -v $(pwd)/atlas.toml:/etc/atlas/atlas.toml:ro \
  --env-file .env \
  --entrypoint /bin/bash \
  atlas:latest

# Inside container
atlas validate-config -c /etc/atlas/atlas.toml
atlas export -c /etc/atlas/atlas.toml --dry-run
```

## Docker Compose

### docker-compose.yml

Create `docker-compose.yml`:

```yaml
version: '3.8'

services:
  atlas:
    image: atlas:latest
    container_name: atlas
    restart: unless-stopped
    
    volumes:
      - ./atlas.toml:/etc/atlas/atlas.toml:ro
      - ./logs:/var/log/atlas
    
    environment:
      - ATLAS_OPENEHR_PASSWORD=${ATLAS_OPENEHR_PASSWORD}
      - ATLAS_COSMOS_KEY=${ATLAS_COSMOS_KEY}
    
    command: export -c /etc/atlas/atlas.toml
    
    # Resource limits
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 4G
        reservations:
          cpus: '1'
          memory: 2G
    
    # Logging
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"
```

### Run with Docker Compose

```bash
# Start service
docker-compose up -d

# View logs
docker-compose logs -f

# Stop service
docker-compose down

# Restart service
docker-compose restart
```

### Scheduled Exports with Docker Compose

Use a cron-like scheduler container:

```yaml
version: '3.8'

services:
  atlas-scheduler:
    image: mcuadros/ofelia:latest
    container_name: atlas-scheduler
    depends_on:
      - atlas
    command: daemon --docker
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro
    labels:
      ofelia.job-run.atlas-export.schedule: "0 0 2 * * *"  # Daily at 2 AM
      ofelia.job-run.atlas-export.container: "atlas"
      ofelia.job-run.atlas-export.command: "atlas export -c /etc/atlas/atlas.toml"

  atlas:
    image: atlas:latest
    container_name: atlas
    volumes:
      - ./atlas.toml:/etc/atlas/atlas.toml:ro
      - ./logs:/var/log/atlas
    environment:
      - ATLAS_OPENEHR_PASSWORD=${ATLAS_OPENEHR_PASSWORD}
      - ATLAS_COSMOS_KEY=${ATLAS_COSMOS_KEY}
    command: ["--help"]  # Default command (overridden by scheduler)
```

## Configuration Management

### Configuration File

Mount configuration as read-only volume:

```bash
docker run --rm \
  -v $(pwd)/atlas.toml:/etc/atlas/atlas.toml:ro \
  atlas:latest export -c /etc/atlas/atlas.toml
```

### Environment Variables

Pass environment variables:

```bash
docker run --rm \
  -e ATLAS_OPENEHR_PASSWORD=password \
  -e ATLAS_COSMOS_KEY=key \
  -e ATLAS_LOG_LEVEL=debug \
  atlas:latest export
```

### Docker Secrets (Swarm)

For Docker Swarm deployments:

```bash
# Create secrets
echo "your-password" | docker secret create atlas_openehr_password -
echo "your-key" | docker secret create atlas_cosmos_key -

# Use in service
docker service create \
  --name atlas \
  --secret atlas_openehr_password \
  --secret atlas_cosmos_key \
  atlas:latest
```

Update configuration to read from secrets:
```toml
[openehr]
password = "${ATLAS_OPENEHR_PASSWORD:-/run/secrets/atlas_openehr_password}"

[cosmosdb]
key = "${ATLAS_COSMOS_KEY:-/run/secrets/atlas_cosmos_key}"
```

## Persistent Storage

### Volume Types

**Bind Mount** (development):
```bash
docker run --rm \
  -v $(pwd)/logs:/var/log/atlas \
  atlas:latest
```

**Named Volume** (production):
```bash
# Create volume
docker volume create atlas-logs

# Use volume
docker run --rm \
  -v atlas-logs:/var/log/atlas \
  atlas:latest
```

### Backup Volumes

```bash
# Backup logs
docker run --rm \
  -v atlas-logs:/data \
  -v $(pwd)/backup:/backup \
  ubuntu tar czf /backup/atlas-logs-$(date +%Y%m%d).tar.gz /data

# Restore logs
docker run --rm \
  -v atlas-logs:/data \
  -v $(pwd)/backup:/backup \
  ubuntu tar xzf /backup/atlas-logs-20250115.tar.gz -C /
```

## Monitoring and Logging

### View Container Logs

```bash
# Follow logs
docker logs -f atlas

# Last 100 lines
docker logs --tail 100 atlas

# Logs since timestamp
docker logs --since 2025-01-15T10:00:00 atlas
```

### Log Drivers

**JSON File** (default):
```yaml
services:
  atlas:
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"
```

**Syslog**:
```yaml
services:
  atlas:
    logging:
      driver: "syslog"
      options:
        syslog-address: "tcp://192.168.0.42:514"
        tag: "atlas"
```

**Fluentd**:
```yaml
services:
  atlas:
    logging:
      driver: "fluentd"
      options:
        fluentd-address: "localhost:24224"
        tag: "atlas"
```

### Health Checks

Add health check to Dockerfile:

```dockerfile
HEALTHCHECK --interval=5m --timeout=10s --retries=3 \
  CMD atlas status -c /etc/atlas/atlas.toml || exit 1
```

Or in docker-compose.yml:

```yaml
services:
  atlas:
    healthcheck:
      test: ["CMD", "atlas", "status", "-c", "/etc/atlas/atlas.toml"]
      interval: 5m
      timeout: 10s
      retries: 3
      start_period: 30s
```

### Resource Monitoring

```bash
# Monitor resource usage
docker stats atlas

# Inspect container
docker inspect atlas

# View resource limits
docker inspect atlas | jq '.[0].HostConfig.Memory'
```

### Graceful Shutdown

Atlas supports graceful shutdown in Docker containers, ensuring data integrity when containers are stopped:

**How It Works:**

1. `docker stop` sends SIGTERM to the Atlas process
2. Atlas completes the current batch being processed
3. Watermarks are saved to the database with `Interrupted` status
4. Atlas exits with code 143 (SIGTERM)
5. Next container run resumes from the checkpoint

**Configuration:**

Docker's default stop timeout is 10 seconds. For Atlas, you should increase this to match your `shutdown_timeout_secs` configuration:

```bash
# Stop with custom timeout (30 seconds)
docker stop --time 30 atlas

# Or set in docker-compose.yml
services:
  atlas:
    stop_grace_period: 30s  # Match export.shutdown_timeout_secs
```

In your `atlas.toml`:

```toml
[export]
# Should match Docker stop_grace_period
shutdown_timeout_secs = 30
```

**Best Practices:**

```yaml
version: '3.8'

services:
  atlas:
    image: atlas:latest

    # Graceful shutdown configuration
    stop_grace_period: 30s  # Allow 30s for graceful shutdown
    stop_signal: SIGTERM    # Use SIGTERM (default)

    volumes:
      - ./atlas.toml:/etc/atlas/atlas.toml:ro
      - ./logs:/var/log/atlas

    environment:
      - ATLAS_OPENEHR_PASSWORD=${ATLAS_OPENEHR_PASSWORD}
      - ATLAS_COSMOS_KEY=${ATLAS_COSMOS_KEY}

    command: export -c /etc/atlas/atlas.toml
```

**Handling Container Stops:**

```bash
# Graceful stop (recommended)
docker stop atlas
# Waits up to stop_grace_period for container to exit

# Force stop (NOT recommended - may lose progress)
docker kill atlas
# Immediately kills container without saving watermarks

# Check exit code
docker inspect atlas --format='{{.State.ExitCode}}'
# 0 = success, 143 = SIGTERM (graceful shutdown)
```

**Monitoring Shutdown:**

```bash
# Watch container logs during shutdown
docker logs -f atlas

# Expected output on Ctrl+C or docker stop:
# ⚠️  Shutdown signal received, completing current batch...
# ⚠️  Export interrupted gracefully. Progress saved.
#    Run the same command to resume from checkpoint.
```

## Production Deployment

### Best Practices

1. **Use specific version tags**:
   ```bash
   docker run atlas:2.4.0  # Not atlas:latest
   ```

2. **Set resource limits**:
   ```bash
   docker run --rm \
     --memory=4g \
     --cpus=2 \
     atlas:2.4.0
   ```

3. **Use read-only root filesystem**:
   ```bash
   docker run --rm \
     --read-only \
     -v atlas-logs:/var/log/atlas \
     atlas:2.4.0
   ```

4. **Run as non-root user** (already configured in Dockerfile)

5. **Use secrets for sensitive data**

### Production docker-compose.yml

```yaml
version: '3.8'

services:
  atlas:
    image: atlas:2.4.0
    container_name: atlas-prod
    restart: unless-stopped

    volumes:
      - ./atlas.toml:/etc/atlas/atlas.toml:ro
      - atlas-logs:/var/log/atlas
    
    environment:
      - ATLAS_OPENEHR_PASSWORD=${ATLAS_OPENEHR_PASSWORD}
      - ATLAS_COSMOS_KEY=${ATLAS_COSMOS_KEY}
    
    command: export -c /etc/atlas/atlas.toml
    
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 4G
        reservations:
          cpus: '1'
          memory: 2G
    
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "5"
    
    healthcheck:
      test: ["CMD", "atlas", "status", "-c", "/etc/atlas/atlas.toml"]
      interval: 5m
      timeout: 10s
      retries: 3
    
    security_opt:
      - no-new-privileges:true
    
    read_only: true
    
    tmpfs:
      - /tmp

volumes:
  atlas-logs:
    driver: local
```

### Container Registry

Push to registry:

```bash
# Tag image
docker tag atlas:2.4.0 your-registry.azurecr.io/atlas:2.4.0

# Login to Azure Container Registry
az acr login --name your-registry

# Push image
docker push your-registry.azurecr.io/atlas:2.4.0

# Pull on production server
docker pull your-registry.azurecr.io/atlas:2.4.0
```

## Troubleshooting

### Container Exits Immediately

**Problem**: Container starts and exits

**Solution**:
```bash
# Check logs
docker logs atlas

# Run with interactive shell
docker run -it --rm --entrypoint /bin/bash atlas:latest

# Check command
docker inspect atlas | jq '.[0].Config.Cmd'
```

### Permission Denied

**Problem**: Cannot write to mounted volumes

**Solution**:
```bash
# Check volume permissions
ls -ld logs/

# Fix permissions
sudo chown -R 1000:1000 logs/

# Or run with user override (not recommended)
docker run --user root atlas:latest
```

### Out of Memory

**Problem**: Container killed due to OOM

**Solution**:
```bash
# Increase memory limit
docker run --memory=8g atlas:latest

# Or reduce batch size in configuration
```

### Network Issues

**Problem**: Cannot connect to OpenEHR or Cosmos DB

**Solution**:
```bash
# Test network from container
docker run -it --rm atlas:latest /bin/bash
curl https://your-ehrbase-server.com

# Check DNS
docker run -it --rm atlas:latest /bin/bash
nslookup your-ehrbase-server.com

# Use host network (debugging only)
docker run --network host atlas:latest
```

---

For more information, see:
- [Standalone Deployment](standalone.md) - Binary deployment
- [Kubernetes Deployment](kubernetes.md) - Kubernetes/AKS deployment
- [User Guide](../user-guide.md) - Usage instructions
- [Configuration Guide](../configuration.md) - Configuration reference

