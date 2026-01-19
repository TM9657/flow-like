---
title: kubectl Basics
description: Essential kubectl commands for managing Flow-Like on Kubernetes.
sidebar:
  order: 15
---

This guide covers the essential `kubectl` commands you'll need to manage Flow-Like on Kubernetes. No prior Kubernetes experience required.

## What is kubectl?

`kubectl` (pronounced "kube-control" or "kube-cuddle") is the command-line tool for interacting with Kubernetes clusters. Think of it as your remote control for the cluster.

---

## Connecting to Your Cluster

### Check Connection

```bash
# See which cluster you're connected to
kubectl config current-context

# List all available clusters
kubectl config get-contexts

# Switch to a different cluster
kubectl config use-context <context-name>
```

### Verify Access

```bash
# List all namespaces (should show 'flow-like')
kubectl get namespaces

# If you don't see flow-like, you may need to install it first
```

---

## Working with Namespaces

Flow-Like runs in its own namespace called `flow-like`. Always specify `-n flow-like` or set it as your default.

```bash
# Set flow-like as your default namespace
kubectl config set-context --current --namespace=flow-like

# Now you don't need -n flow-like every time
kubectl get pods  # Same as: kubectl get pods -n flow-like
```

---

## Viewing Resources

### List All Flow-Like Components

```bash
# All pods (running containers)
kubectl get pods -n flow-like

# All services (network endpoints)
kubectl get services -n flow-like

# All deployments (manages pods)
kubectl get deployments -n flow-like

# Everything at once
kubectl get all -n flow-like
```

### Example Output

```
NAME                                      READY   STATUS    RESTARTS   AGE
pod/flow-like-api-7d4b8c9f5-abc12         1/1     Running   0          2h
pod/flow-like-api-7d4b8c9f5-def34         1/1     Running   0          2h
pod/flow-like-executor-pool-6ddf4bdc5-x1  1/1     Running   0          2h
pod/flow-like-cockroachdb-0               1/1     Running   0          2h
pod/flow-like-redis-master-0              1/1     Running   0          2h

NAME                          TYPE        CLUSTER-IP      PORT(S)
service/flow-like-api         ClusterIP   10.43.100.50    8080/TCP
service/flow-like-cockroachdb ClusterIP   10.43.100.51    26257/TCP
service/flow-like-redis       ClusterIP   10.43.100.52    6379/TCP
```

### Detailed Information

```bash
# Describe a specific pod (shows events and config)
kubectl describe pod <pod-name> -n flow-like

# Describe the API deployment
kubectl describe deployment flow-like-api -n flow-like

# See resource usage (CPU/memory)
kubectl top pods -n flow-like
```

---

## Accessing the API

### Port Forwarding (Development)

Port forwarding creates a tunnel from your local machine to a service in the cluster.

```bash
# Forward local port 8083 to the API service port 8080
kubectl port-forward svc/flow-like-api 8083:8080 -n flow-like

# Now open another terminal and test:
curl http://localhost:8083/api/v1/health
```

:::tip
Add `&` at the end to run in the background:
```bash
kubectl port-forward svc/flow-like-api 8083:8080 -n flow-like &
```
:::

### Port Forwarding Breakdown

```
kubectl port-forward svc/flow-like-api 8083:8080 -n flow-like
                     │                  │    │       │
                     │                  │    │       └─ Namespace
                     │                  │    └─ Service port (inside cluster)
                     │                  └─ Your local port
                     └─ Service name
```

### Other Services You Might Access

```bash
# CockroachDB Admin UI (port 8080)
kubectl port-forward svc/flow-like-cockroachdb 8084:8080 -n flow-like
# Open http://localhost:8084

# Redis (port 6379)
kubectl port-forward svc/flow-like-redis-master 6379:6379 -n flow-like
```

---

## Viewing Logs

### Basic Log Commands

```bash
# Logs from the API (last 100 lines)
kubectl logs deployment/flow-like-api -n flow-like --tail=100

# Follow logs in real-time (like tail -f)
kubectl logs deployment/flow-like-api -n flow-like -f

# Logs from a specific pod
kubectl logs flow-like-api-7d4b8c9f5-abc12 -n flow-like
```

### When There Are Multiple Containers

```bash
# List containers in a pod
kubectl get pod <pod-name> -n flow-like -o jsonpath='{.spec.containers[*].name}'

# Logs from a specific container
kubectl logs <pod-name> -c <container-name> -n flow-like
```

### View Previous Logs (after restart)

```bash
# Logs from the previous container instance
kubectl logs deployment/flow-like-api -n flow-like --previous
```

---

## Restarting Services

### Restart a Deployment

```bash
# Graceful restart (rolling)
kubectl rollout restart deployment/flow-like-api -n flow-like

# Watch the restart progress
kubectl rollout status deployment/flow-like-api -n flow-like
```

### Delete a Pod (Kubernetes recreates it)

```bash
# Force a specific pod to restart
kubectl delete pod flow-like-api-7d4b8c9f5-abc12 -n flow-like

# Kubernetes automatically creates a new pod to replace it
```

---

## Debugging Problems

### Check Pod Status

```bash
# Quick overview
kubectl get pods -n flow-like

# Common statuses:
# Running     - Everything is good
# Pending     - Waiting for resources
# CrashLoopBackOff - Container keeps crashing
# ImagePullBackOff - Can't download the container image
# Error       - Container exited with error
```

### Investigate a Failing Pod

```bash
# See events and conditions
kubectl describe pod <pod-name> -n flow-like

# Look at the end for "Events:" section
# Common issues:
# - FailedScheduling: Not enough CPU/memory
# - ImagePullBackOff: Wrong image name or no access
# - CrashLoopBackOff: Check logs for errors
```

### Shell Into a Running Container

```bash
# Open a shell inside the API container
kubectl exec -it deployment/flow-like-api -n flow-like -- /bin/sh

# Run a single command
kubectl exec deployment/flow-like-api -n flow-like -- cat /app/config.yaml
```

### Test from Inside the Cluster

```bash
# Create a temporary debug pod
kubectl run debug --image=curlimages/curl -it --rm -n flow-like -- sh

# Inside the pod, test internal services:
curl http://flow-like-api:8080/api/v1/health
curl http://flow-like-cockroachdb:26257
```

---

## Configuration and Secrets

### View ConfigMaps

```bash
# List all configmaps
kubectl get configmaps -n flow-like

# View a specific configmap
kubectl describe configmap flow-like-api -n flow-like

# Get the raw data
kubectl get configmap flow-like-api -n flow-like -o yaml
```

### View Secrets (names only for security)

```bash
# List secrets
kubectl get secrets -n flow-like

# View secret structure (values are base64 encoded)
kubectl get secret flow-like-storage -n flow-like -o yaml
```

### Decode a Secret Value

```bash
# Get and decode a specific value
kubectl get secret flow-like-storage -n flow-like \
  -o jsonpath='{.data.access-key-id}' | base64 -d
```

---

## Scaling

### Manual Scaling

```bash
# Scale API to 5 replicas
kubectl scale deployment/flow-like-api --replicas=5 -n flow-like

# Scale down to 1 replica
kubectl scale deployment/flow-like-api --replicas=1 -n flow-like
```

### Check Autoscaler

```bash
# View horizontal pod autoscaler
kubectl get hpa -n flow-like

# Detailed autoscaler status
kubectl describe hpa flow-like-api -n flow-like
```

---

## Helm Commands

Helm is used to install and upgrade Flow-Like.

```bash
# List installed releases
helm list -n flow-like

# View current values
helm get values flow-like -n flow-like

# Upgrade with new values
helm upgrade flow-like ./helm -n flow-like -f values.yaml

# Rollback to previous version
helm rollback flow-like -n flow-like

# Uninstall
helm uninstall flow-like -n flow-like
```

---

## Quick Reference Card

| Task | Command |
|------|---------|
| List pods | `kubectl get pods -n flow-like` |
| View logs | `kubectl logs deploy/flow-like-api -n flow-like` |
| Follow logs | `kubectl logs deploy/flow-like-api -n flow-like -f` |
| Port forward | `kubectl port-forward svc/flow-like-api 8083:8080 -n flow-like` |
| Restart | `kubectl rollout restart deploy/flow-like-api -n flow-like` |
| Describe | `kubectl describe pod <name> -n flow-like` |
| Shell access | `kubectl exec -it deploy/flow-like-api -n flow-like -- sh` |
| Scale | `kubectl scale deploy/flow-like-api --replicas=3 -n flow-like` |

---

## Common Issues

### "No resources found"
- Check if you're in the right namespace: `-n flow-like`
- Check if Flow-Like is installed: `helm list -n flow-like`

### "Connection refused"
- Pod might not be running: `kubectl get pods -n flow-like`
- Wrong port: Check `kubectl get svc -n flow-like`

### "Pod stuck in Pending"
- Not enough resources: `kubectl describe pod <name> -n flow-like`
- Check cluster capacity: `kubectl top nodes`

### "CrashLoopBackOff"
- Application is crashing. Check logs: `kubectl logs <pod> -n flow-like`
- Check previous logs: `kubectl logs <pod> -n flow-like --previous`

---

## Next Steps

- [API Reference](/self-hosting/kubernetes/api-reference/) — All available endpoints
- [Configuration](/self-hosting/kubernetes/configuration/) — Environment variables
- [Local Development](/self-hosting/kubernetes/local-development/) — Using k3d
