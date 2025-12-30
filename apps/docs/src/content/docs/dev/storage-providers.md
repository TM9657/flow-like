---
title: Storage Providers Setup
description: Setting up cloud storage providers for local development and testing
sidebar:
  order: 15
---

This guide explains how to set up cloud storage providers for local development and testing of Flow-Like.

## Overview

Flow-Like supports multiple storage backends:
- **AWS S3** (with S3 Express One Zone for metadata)
- **Azure Blob Storage** (ADLS Gen2 with Directory SAS)
- **Google Cloud Storage**
- **MinIO** (S3-compatible, self-hosted)

The API uses **runtime credentials** that derive scoped, temporary credentials from master credentials, ensuring each user/app only has access to their designated paths.

## Running Unit Tests

Unit tests for credentials test serialization/deserialization and don't require cloud resources:

```bash
# Core package - shared credentials
cargo test -p flow-like --lib credentials

# API package - runtime credentials (requires feature flags)
cargo test -p flow-like-api --features "full" --lib credentials
```

## Running Integration Tests

Integration tests verify actual cloud access and require environment setup:

```bash
# Run all ignored (integration) tests
cargo test -p flow-like-api --features "full" -- --ignored

# Run tests for a specific provider
cargo test -p flow-like-api --features "aws" -- --ignored aws_tests
cargo test -p flow-like-api --features "azure" -- --ignored azure_tests
cargo test -p flow-like-api --features "gcp" -- --ignored gcp_tests
cargo test -p flow-like-api --features "minio" -- --ignored minio_tests
```

---

## AWS S3 Setup

### Required Resources

1. **S3 Buckets**:
   - Meta bucket (S3 Express One Zone recommended for low-latency metadata)
   - Content bucket (standard S3)

2. **IAM Role** with:
   - `s3:*` permissions on both buckets
   - Trust policy allowing AssumeRole from your service

### Environment Variables

```bash
export AWS_ACCESS_KEY_ID="AKIAIOSFODNN7EXAMPLE"
export AWS_SECRET_ACCESS_KEY="wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
export AWS_REGION="us-west-2"
export META_BUCKET="your-meta-bucket--usw2-az1--x-s3"
export CONTENT_BUCKET="your-content-bucket"
export RUNTIME_ROLE_ARN="arn:aws:iam::123456789012:role/FlowLikeRuntimeRole"
```

### Creating S3 Express One Zone Bucket

```bash
aws s3api create-bucket \
    --bucket your-meta-bucket--usw2-az1--x-s3 \
    --region us-west-2 \
    --create-bucket-configuration \
    "Location={Type=AvailabilityZone,Name=usw2-az1},Bucket={DataRedundancy=SingleAvailabilityZone,Type=Directory}"
```

### IAM Role Policy

Create a role with this trust policy:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": {
        "AWS": "arn:aws:iam::YOUR_ACCOUNT:user/your-user"
      },
      "Action": "sts:AssumeRole"
    }
  ]
}
```

And attach this permissions policy:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": "s3:*",
      "Resource": [
        "arn:aws:s3:::your-content-bucket",
        "arn:aws:s3:::your-content-bucket/*",
        "arn:aws:s3:::your-meta-bucket--usw2-az1--x-s3",
        "arn:aws:s3:::your-meta-bucket--usw2-az1--x-s3/*"
      ]
    },
    {
      "Effect": "Allow",
      "Action": "s3express:CreateSession",
      "Resource": "*"
    }
  ]
}
```

---

## Azure Blob Storage Setup

### Required Resources

1. **Storage Account** with hierarchical namespace enabled (ADLS Gen2)
2. **Containers**: meta-container, content-container

### Environment Variables

```bash
export AZURE_STORAGE_ACCOUNT_NAME="mystorageaccount"
export AZURE_STORAGE_ACCOUNT_KEY="base64encodedkey..."
export AZURE_META_CONTAINER="meta-container"
export AZURE_CONTENT_CONTAINER="content-container"
```

### Create Storage Account

```bash
az storage account create \
    --name mystorageaccount \
    --resource-group myresourcegroup \
    --location westus2 \
    --sku Standard_LRS \
    --kind StorageV2 \
    --enable-hierarchical-namespace true

# Create containers
az storage container create --name meta-container --account-name mystorageaccount
az storage container create --name content-container --account-name mystorageaccount
```

### How Directory SAS Works

The API generates Directory SAS tokens scoped to specific paths:
- `apps/{app_id}/*` for app data
- `users/{user_id}/apps/{app_id}/*` for user data
- `logs/runs/{app_id}/*` for logs

SAS tokens include:
- Time-limited validity (1 hour default)
- Path restrictions
- Permission restrictions (read-only vs read-write)

---

## Google Cloud Storage Setup

### Required Resources

1. **GCS Buckets**: meta-bucket, content-bucket
2. **Service Account** with `roles/storage.admin`

### Environment Variables

```bash
# Option 1: Path to key file
export GOOGLE_APPLICATION_CREDENTIALS="/path/to/service-account-key.json"

# Option 2: Key content directly (for containers)
export GOOGLE_APPLICATION_CREDENTIALS_JSON='{"type":"service_account",...}'

export GCP_META_BUCKET="my-meta-bucket"
export GCP_CONTENT_BUCKET="my-content-bucket"
```

### Create Service Account

```bash
# Create service account
gcloud iam service-accounts create flow-like-storage \
    --description="Storage account for Flow-Like" \
    --display-name="Flow-Like Storage"

# Grant permissions
gcloud projects add-iam-policy-binding YOUR_PROJECT \
    --member="serviceAccount:flow-like-storage@YOUR_PROJECT.iam.gserviceaccount.com" \
    --role="roles/storage.admin"

# Create key
gcloud iam service-accounts keys create service-account-key.json \
    --iam-account=flow-like-storage@YOUR_PROJECT.iam.gserviceaccount.com
```

### Service Account Key Format

```json
{
  "type": "service_account",
  "project_id": "my-project-id",
  "private_key_id": "key-id",
  "private_key": "-----BEGIN RSA PRIVATE KEY-----\n...\n-----END RSA PRIVATE KEY-----\n",
  "client_email": "flow-like-storage@my-project.iam.gserviceaccount.com",
  "client_id": "123456789",
  "auth_uri": "https://accounts.google.com/o/oauth2/auth",
  "token_uri": "https://oauth2.googleapis.com/token"
}
```

---

## MinIO Setup (Local Development)

MinIO is ideal for local development - it's S3-compatible and easy to set up.

### Quick Start with Docker

```bash
docker run -d \
  --name minio \
  -p 9000:9000 \
  -p 9001:9001 \
  -e "MINIO_ROOT_USER=minioadmin" \
  -e "MINIO_ROOT_PASSWORD=minioadmin123" \
  quay.io/minio/minio server /data --console-address ":9001"
```

### Environment Variables

```bash
export MINIO_ENDPOINT="http://localhost:9000"
export MINIO_ACCESS_KEY_ID="minioadmin"
export MINIO_SECRET_ACCESS_KEY="minioadmin123"
export MINIO_REGION="us-east-1"
export MINIO_META_BUCKET="meta-bucket"
export MINIO_CONTENT_BUCKET="content-bucket"
export MINIO_RUNTIME_ROLE_ARN="arn:minio:iam:::role/FlowLikeRole"
```

### Create Buckets

```bash
# Install MinIO Client
brew install minio/stable/mc  # macOS
# or download from https://min.io/docs/minio/linux/reference/minio-mc.html

# Configure alias
mc alias set local http://localhost:9000 minioadmin minioadmin123

# Create buckets
mc mb local/meta-bucket
mc mb local/content-bucket
```

### AssumeRole Setup

For scoped credentials, MinIO needs STS (Security Token Service) configured:

1. **Enable STS in MinIO**:
```bash
mc admin config set local identity_openid --env
# Or use built-in STS with policy-based access
```

2. **Create a policy**:
```bash
cat > flow-like-policy.json << 'EOF'
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": ["s3:*"],
      "Resource": ["arn:aws:s3:::*"]
    }
  ]
}
EOF

mc admin policy create local flow-like-policy flow-like-policy.json
mc admin policy attach local flow-like-policy --user minioadmin
```

---

## API Feature Flags

Build the API with specific storage providers:

```bash
# Single provider
cargo build -p flow-like-api --features "aws"
cargo build -p flow-like-api --features "azure"
cargo build -p flow-like-api --features "gcp"
cargo build -p flow-like-api --features "minio"

# Multiple providers (priority: AWS → Azure → GCP → MinIO)
cargo build -p flow-like-api --features "aws,azure"

# All providers
cargo build -p flow-like-api --features "full"
```

---

## Credential Scoping

The API derives scoped credentials with path-based restrictions:

| Access Mode | Allowed Paths | Permissions |
|-------------|---------------|-------------|
| `EditApp` | `apps/{app_id}/*` | Read, Write, Delete |
| `ReadApp` | `apps/{app_id}/*` | Read only |
| `InvokeNone` | `users/{sub}/apps/{app_id}/*`, `tmp/user/{sub}/apps/{app_id}/*` | Read, Write |
| `InvokeRead` | All app + user paths | Read only |
| `InvokeWrite` | All app + user paths | Read, Write |
| `ReadLogs` | `logs/runs/{app_id}/*` | Read only |

### Security Model

1. **Master credentials** are stored securely on the API server
2. **Scoped credentials** are generated per-request with:
   - Limited time validity (1 hour)
   - Path restrictions based on user/app context
   - Permission restrictions based on operation type
3. **Desktop app** receives only scoped credentials, never master credentials

---

## Troubleshooting

### AWS

| Error | Solution |
|-------|----------|
| `InvalidAccessKeyId` | Verify AWS_ACCESS_KEY_ID is correct and active |
| `AccessDenied` | Check IAM role permissions and trust policy |
| `ExpiredToken` | Session token expired, refresh credentials |
| S3 Express errors | Ensure bucket name follows `bucket--az-id--x-s3` format |

### Azure

| Error | Solution |
|-------|----------|
| `AuthorizationFailure` | SAS token expired or missing permissions |
| `InvalidResourceName` | Container names must be lowercase, 3-63 chars |
| `AuthenticationFailed` | Verify account key is correct |

### GCP

| Error | Solution |
|-------|----------|
| `Permission denied` | Service account lacks storage permissions |
| `Invalid JSON` | Verify service account key format |
| `Project not found` | Check project ID in credentials |

### MinIO

| Error | Solution |
|-------|----------|
| `Connection refused` | MinIO server not running, check endpoint |
| `Access Denied` | Verify credentials and bucket policies |
| `NoSuchBucket` | Create bucket with `mc mb` |
| Path-style errors | Ensure `virtual_hosted_style_request(false)` |

---

## CI/CD Secrets

For automated testing, configure these secrets:

| Secret | Description |
|--------|-------------|
| `AWS_ACCESS_KEY_ID` | AWS access key |
| `AWS_SECRET_ACCESS_KEY` | AWS secret key |
| `AWS_REGION` | AWS region |
| `RUNTIME_ROLE_ARN` | IAM role ARN |
| `META_BUCKET` | S3 meta bucket |
| `CONTENT_BUCKET` | S3 content bucket |
| `AZURE_STORAGE_ACCOUNT_NAME` | Azure storage account |
| `AZURE_STORAGE_ACCOUNT_KEY` | Azure storage key |
| `AZURE_META_CONTAINER` | Azure meta container |
| `AZURE_CONTENT_CONTAINER` | Azure content container |
| `GOOGLE_APPLICATION_CREDENTIALS_JSON` | Base64 service account JSON |
| `GCP_META_BUCKET` | GCS meta bucket |
| `GCP_CONTENT_BUCKET` | GCS content bucket |
