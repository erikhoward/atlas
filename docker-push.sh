#!/bin/bash
# Atlas Docker Image Build and Push Script
# This script builds and pushes the Atlas Docker image to Docker Hub
#
# Usage:
#   ./docker-push.sh <version>
#
# Example:
#   ./docker-push.sh 1.0.0
#
# Prerequisites:
#   - Docker installed and running
#   - Logged in to Docker Hub (docker login)
#   - DOCKERHUB_USERNAME environment variable set (or modify script)

set -e  # Exit on error

# Configuration
DOCKERHUB_USERNAME="${DOCKERHUB_USERNAME:-erikhoward}"
IMAGE_NAME="atlas"
FULL_IMAGE_NAME="${DOCKERHUB_USERNAME}/${IMAGE_NAME}"

# Check if version argument is provided
if [ -z "$1" ]; then
    echo "Error: Version argument required"
    echo "Usage: $0 <version>"
    echo "Example: $0 1.0.0"
    exit 1
fi

VERSION="$1"

# Validate version format (basic semver check)
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Warning: Version '$VERSION' does not follow semantic versioning (x.y.z)"
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

echo "=========================================="
echo "Atlas Docker Build and Push"
echo "=========================================="
echo "Image: ${FULL_IMAGE_NAME}"
echo "Version: ${VERSION}"
echo "=========================================="
echo

# Build the Docker image
echo "Building Docker image..."
docker build -t "${FULL_IMAGE_NAME}:${VERSION}" .

# Tag as latest
echo "Tagging as latest..."
docker tag "${FULL_IMAGE_NAME}:${VERSION}" "${FULL_IMAGE_NAME}:latest"

# Show built images
echo
echo "Built images:"
docker images | grep "${IMAGE_NAME}" | head -5

# Confirm before pushing
echo
read -p "Push images to Docker Hub? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted. Images built locally but not pushed."
    exit 0
fi

# Push version tag
echo "Pushing ${FULL_IMAGE_NAME}:${VERSION}..."
docker push "${FULL_IMAGE_NAME}:${VERSION}"

# Push latest tag
echo "Pushing ${FULL_IMAGE_NAME}:latest..."
docker push "${FULL_IMAGE_NAME}:latest"

echo
echo "=========================================="
echo "✓ Successfully pushed to Docker Hub!"
echo "=========================================="
echo "Images available at:"
echo "  - ${FULL_IMAGE_NAME}:${VERSION}"
echo "  - ${FULL_IMAGE_NAME}:latest"
echo
echo "Pull with:"
echo "  docker pull ${FULL_IMAGE_NAME}:${VERSION}"
echo "  docker pull ${FULL_IMAGE_NAME}:latest"
echo "=========================================="

