#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Default values
NAMESPACE="${NAMESPACE:-flow-like}"
RELEASE="${RELEASE:-flow-like}"
VALUES="${VALUES:-$ROOT_DIR/helm/values.yaml}"

echo "Deploying Flow-Like to Kubernetes..."
echo "Namespace: $NAMESPACE"
echo "Release: $RELEASE"

# Create namespace if it doesn't exist
kubectl create namespace "$NAMESPACE" --dry-run=client -o yaml | kubectl apply -f -

# Deploy with Helm
helm upgrade --install "$RELEASE" "$ROOT_DIR/helm" \
    --namespace "$NAMESPACE" \
    --values "$VALUES" \
    "$@"

echo ""
echo "Deployment complete!"
echo ""
echo "To check status:"
echo "  kubectl get pods -n $NAMESPACE"
echo ""
echo "To view logs:"
echo "  kubectl logs -n $NAMESPACE -l app.kubernetes.io/name=flow-like -f"
