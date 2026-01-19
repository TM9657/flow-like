#!/usr/bin/env bash
# =============================================================================
# Flow-Like Kubernetes - Create Secrets and ConfigMaps
# =============================================================================
# This script creates the necessary Kubernetes secrets and configmaps from
# environment variables or .env file
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NAMESPACE="${K8S_NAMESPACE:-flow-like}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Load .env file if it exists
if [[ -f "$SCRIPT_DIR/../.env" ]]; then
    log_info "Loading environment from .env file..."
    set -a
    source "$SCRIPT_DIR/../.env"
    set +a
fi

# Check required variables
check_required() {
    local var_name=$1
    if [[ -z "${!var_name:-}" ]]; then
        log_error "Required environment variable $var_name is not set"
        exit 1
    fi
}

# Create namespace if it doesn't exist
create_namespace() {
    if ! kubectl get namespace "$NAMESPACE" &>/dev/null; then
        log_info "Creating namespace: $NAMESPACE"
        kubectl create namespace "$NAMESPACE"
    else
        log_info "Namespace $NAMESPACE already exists"
    fi
}

# Create database secret
create_db_secret() {
    log_info "Creating database secret..."

    local db_url="${DATABASE_URL:-postgresql://${POSTGRES_USER:-flowlike}:${POSTGRES_PASSWORD:-flowlike_dev}@${POSTGRES_HOST:-postgres}:${POSTGRES_PORT:-5432}/${POSTGRES_DB:-flowlike}}"

    kubectl create secret generic flow-like-db \
        --namespace="$NAMESPACE" \
        --from-literal=DATABASE_URL="$db_url" \
        --dry-run=client -o yaml | kubectl apply -f -

    log_info "Database secret created/updated"
}

# Create S3 secret
create_s3_secret() {
    log_info "Creating S3 secret..."

    kubectl create secret generic flow-like-s3 \
        --namespace="$NAMESPACE" \
        --from-literal=S3_ENDPOINT="${S3_ENDPOINT}" \
        --from-literal=S3_REGION="${S3_REGION:-us-east-1}" \
        --from-literal=S3_ACCESS_KEY_ID="${S3_ACCESS_KEY_ID}" \
        --from-literal=S3_SECRET_ACCESS_KEY="${S3_SECRET_ACCESS_KEY}" \
        --from-literal=META_BUCKET_NAME="${META_BUCKET_NAME:-${META_BUCKET:-flow-like-meta}}" \
        --from-literal=CONTENT_BUCKET_NAME="${CONTENT_BUCKET_NAME:-${CONTENT_BUCKET:-flow-like-content}}" \
        --from-literal=META_BUCKET="${META_BUCKET_NAME:-${META_BUCKET:-flow-like-meta}}" \
        --from-literal=CONTENT_BUCKET="${CONTENT_BUCKET_NAME:-${CONTENT_BUCKET:-flow-like-content}}" \
        --dry-run=client -o yaml | kubectl apply -f -

    log_info "S3 secret created/updated"
}

# Create Redis secret
create_redis_secret() {
    log_info "Creating Redis secret..."

    local redis_url="${REDIS_URL:-redis://${REDIS_HOST:-redis}:${REDIS_PORT:-6379}}"

    kubectl create secret generic flow-like-redis-secret \
        --namespace="$NAMESPACE" \
        --from-literal=REDIS_URL="$redis_url" \
        --dry-run=client -o yaml | kubectl apply -f -

    log_info "Redis secret created/updated"
}

# Create API ConfigMap
create_api_configmap() {
    log_info "Creating API ConfigMap..."

    kubectl create configmap flow-like-api-config \
        --namespace="$NAMESPACE" \
        --from-literal=API_HOST="${API_HOST:-0.0.0.0}" \
        --from-literal=API_PORT="${API_PORT:-8080}" \
        --from-literal=RUST_LOG="${RUST_LOG:-info}" \
        --from-literal=METRICS_ENABLED="${METRICS_ENABLED:-true}" \
        --dry-run=client -o yaml | kubectl apply -f -

    log_info "API ConfigMap created/updated"
}

# Create Executor ConfigMap
create_executor_configmap() {
    log_info "Creating Executor ConfigMap..."

    kubectl create configmap flow-like-executor-config \
        --namespace="$NAMESPACE" \
        --from-literal=K8S_NAMESPACE="$NAMESPACE" \
        --from-literal=EXECUTOR_IMAGE="${EXECUTOR_IMAGE:-flow-like/k8s-executor:latest}" \
        --from-literal=USE_KATA_CONTAINERS="${USE_KATA_CONTAINERS:-false}" \
        --from-literal=JOB_TIMEOUT="${JOB_TIMEOUT:-3600}" \
        --from-literal=MAX_CONCURRENT_JOBS="${MAX_CONCURRENT_JOBS:-100}" \
        --from-literal=RUST_LOG="${RUST_LOG:-info}" \
        --dry-run=client -o yaml | kubectl apply -f -

    log_info "Executor ConfigMap created/updated"
}

# Create optional secrets (OpenAI, OpenRouter, Sentry)
create_optional_secrets() {
    if [[ -n "${OPENAI_API_KEY:-}" ]]; then
        log_info "Creating OpenAI secret..."
        kubectl create secret generic flow-like-openai-secret \
            --namespace="$NAMESPACE" \
            --from-literal=OPENAI_API_KEY="$OPENAI_API_KEY" \
            --from-literal=OPENAI_ENDPOINT="${OPENAI_ENDPOINT:-https://api.openai.com/v1}" \
            --dry-run=client -o yaml | kubectl apply -f -
    fi

    if [[ -n "${OPENROUTER_API_KEY:-}" ]]; then
        log_info "Creating OpenRouter secret..."
        kubectl create secret generic flow-like-openrouter-secret \
            --namespace="$NAMESPACE" \
            --from-literal=OPENROUTER_API_KEY="$OPENROUTER_API_KEY" \
            --from-literal=OPENROUTER_ENDPOINT="${OPENROUTER_ENDPOINT:-https://openrouter.ai/api}" \
            --dry-run=client -o yaml | kubectl apply -f -
    fi

    if [[ -n "${SENTRY_ENDPOINT:-}" ]]; then
        log_info "Creating Sentry secret..."
        kubectl create secret generic flow-like-sentry-secret \
            --namespace="$NAMESPACE" \
            --from-literal=SENTRY_ENDPOINT="$SENTRY_ENDPOINT" \
            --dry-run=client -o yaml | kubectl apply -f -
    fi
}

# Print summary
print_summary() {
    echo ""
    log_info "=== Configuration Summary ==="
    echo "Namespace: $NAMESPACE"
    echo ""
    echo "Secrets created:"
    kubectl get secrets -n "$NAMESPACE" -l app.kubernetes.io/part-of=flow-like 2>/dev/null || \
        kubectl get secrets -n "$NAMESPACE" | grep flow-like || true
    echo ""
    echo "ConfigMaps created:"
    kubectl get configmaps -n "$NAMESPACE" | grep flow-like || true
    echo ""
    log_info "=== Setup Complete ==="
}

# Main
main() {
    log_info "Setting up Flow-Like Kubernetes configuration..."

    create_namespace
    create_db_secret
    create_s3_secret
    create_redis_secret
    create_api_configmap
    create_executor_configmap
    create_optional_secrets
    print_summary
}

main "$@"
