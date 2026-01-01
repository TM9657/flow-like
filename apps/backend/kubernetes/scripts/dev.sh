#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "Starting local development environment..."

# Start dependencies
cd "$ROOT_DIR"
docker-compose up -d

echo "Waiting for services to be ready..."
sleep 5

# Set environment variables for local development
export DATABASE_URL="postgres://flowlike:flowlike_dev@localhost:5432/flowlike"
export REDIS_URL="redis://localhost:6379"
export PORT="8080"
export RUST_LOG="info,k8s_api=debug"
export KUBERNETES_NAMESPACE="flow-like"
export EXECUTOR_IMAGE="flow-like/executor:latest"
export EXECUTOR_RUNTIME_CLASS="kata"

echo ""
echo "Environment ready!"
echo ""
echo "Services:"
echo "  PostgreSQL: localhost:5432"
echo "  Redis:      localhost:6379"
echo ""
echo "To run the API:"
echo "  cd api && cargo run"
echo ""
echo "To stop:"
echo "  docker-compose down"
