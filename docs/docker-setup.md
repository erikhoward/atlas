# Docker Setup Guide

This guide explains how to set up Docker support for Atlas, including building images locally, pushing to Docker Hub, and configuring GitHub Actions for automated builds.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Building Docker Images Locally](#building-docker-images-locally)
- [Pushing to Docker Hub](#pushing-to-docker-hub)
- [GitHub Actions Setup](#github-actions-setup)
- [Running Atlas in Docker](#running-atlas-in-docker)

## Prerequisites

- Docker installed and running (Docker Desktop or Docker Engine)
- Docker Hub account (for pushing images)
- GitHub repository with Actions enabled (for CI/CD)

## Building Docker Images Locally

### Basic Build

Build the Atlas Docker image locally:

```bash
docker build -t atlas:local .
```

This creates a multi-stage Docker image that:
1. Compiles the Rust application in a builder container
2. Creates a minimal runtime image with only the binary and dependencies
3. Results in an optimized image (~100-150 MB)

### Build with Version Tag

```bash
docker build -t atlas:1.0.0 .
```

### Verify the Build

```bash
# List the image
docker images | grep atlas

# Test the image
docker run --rm atlas:local --version
```

## Pushing to Docker Hub

### Manual Push

1. **Log in to Docker Hub:**

```bash
docker login
```

Enter your Docker Hub username and password when prompted.

2. **Tag the Image:**

```bash
# Replace 'yourusername' with your Docker Hub username
docker tag atlas:local yourusername/atlas:1.0.0
docker tag atlas:local yourusername/atlas:latest
```

3. **Push to Docker Hub:**

```bash
docker push yourusername/atlas:1.0.0
docker push yourusername/atlas:latest
```

### Using the Push Script

A convenience script is provided for building and pushing:

```bash
# Make the script executable (if not already)
chmod +x docker-push.sh

# Set your Docker Hub username (optional, defaults to 'erikhoward')
export DOCKERHUB_USERNAME=yourusername

# Build and push
./docker-push.sh 1.0.0
```

The script will:
- Build the Docker image
- Tag it with the version and 'latest'
- Prompt for confirmation before pushing
- Push both tags to Docker Hub

## GitHub Actions Setup

The repository includes a GitHub Actions workflow that automatically builds and pushes Docker images to Docker Hub when code is pushed to the main branch or when version tags are created.

### Required GitHub Secrets

You need to configure two secrets in your GitHub repository:

1. **Navigate to Repository Settings:**
   - Go to your GitHub repository
   - Click on **Settings** → **Secrets and variables** → **Actions**

2. **Add the following secrets:**

   | Secret Name | Description | How to Get It |
   |-------------|-------------|---------------|
   | `DOCKERHUB_USERNAME` | Your Docker Hub username | Your Docker Hub account username |
   | `DOCKERHUB_TOKEN` | Docker Hub access token | Create at https://hub.docker.com/settings/security |

### Creating a Docker Hub Access Token

1. Log in to Docker Hub
2. Go to **Account Settings** → **Security** → **Access Tokens**
3. Click **New Access Token**
4. Give it a description (e.g., "GitHub Actions - Atlas")
5. Set permissions to **Read & Write**
6. Click **Generate**
7. Copy the token (you won't be able to see it again!)
8. Add it as `DOCKERHUB_TOKEN` in GitHub secrets

### Workflow Triggers

The GitHub Actions workflow triggers on:

- **Push to main branch**: Builds and pushes with `latest` tag
- **Version tags** (e.g., `v1.0.0`): Builds and pushes with version tags
- **Pull requests**: Builds only (doesn't push)
- **Manual trigger**: Can be triggered manually from GitHub Actions UI

### Tagging Strategy

The workflow automatically creates the following tags:

| Trigger | Tags Created |
|---------|--------------|
| Push to main | `latest`, `main-<sha>` |
| Tag `v1.2.3` | `1.2.3`, `1.2`, `1`, `latest` |
| Pull request | `pr-<number>` (build only, not pushed) |

### Multi-Platform Support

The GitHub Actions workflow builds images for multiple platforms:
- `linux/amd64` (x86_64)
- `linux/arm64` (ARM64, e.g., Apple Silicon, AWS Graviton)

This ensures the image works on various architectures.

## Running Atlas in Docker

### Basic Usage

```bash
docker run --rm \
  -v $(pwd)/atlas.toml:/app/config/atlas.toml \
  erikhoward/atlas:latest \
  export --config /app/config/atlas.toml
```

### With Environment Variables

```bash
docker run --rm \
  -v $(pwd)/atlas.toml:/app/config/atlas.toml \
  -e ATLAS_OPENEHR_USERNAME=myuser \
  -e ATLAS_OPENEHR_PASSWORD=mypassword \
  -e ATLAS_COSMOSDB_KEY=mycosmoskey \
  erikhoward/atlas:latest \
  export --config /app/config/atlas.toml
```

### With Log Output

```bash
docker run --rm \
  -v $(pwd)/atlas.toml:/app/config/atlas.toml \
  -v $(pwd)/logs:/app/logs \
  -e RUST_LOG=debug \
  erikhoward/atlas:latest \
  export --config /app/config/atlas.toml
```

### Interactive Mode

```bash
docker run -it --rm \
  -v $(pwd)/atlas.toml:/app/config/atlas.toml \
  erikhoward/atlas:latest \
  /bin/bash
```

## Troubleshooting

### Build Fails

- **Issue**: Docker build fails during Rust compilation
- **Solution**: Ensure you have enough disk space and memory allocated to Docker

### Permission Denied

- **Issue**: Container can't write to mounted volumes
- **Solution**: The container runs as user `atlas` (UID 1000). Ensure mounted directories have appropriate permissions:
  ```bash
  chmod 755 logs/
  ```

### GitHub Actions Fails to Push

- **Issue**: Workflow fails with authentication error
- **Solution**: Verify that `DOCKERHUB_USERNAME` and `DOCKERHUB_TOKEN` secrets are correctly set in GitHub

### Image Not Found

- **Issue**: `docker pull` fails with "not found"
- **Solution**: Ensure the image name matches your Docker Hub username and the image has been pushed successfully

## Additional Resources

- [Docker Documentation](https://docs.docker.com/)
- [Docker Hub](https://hub.docker.com/)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Atlas Documentation](../README.md)

