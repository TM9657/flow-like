---
title: Storage Providers
description: Configure object storage for Flow-Like Docker Compose deployment.
sidebar:
  order: 24
---

Flow-Like requires S3-compatible object storage for storing workflow data, execution logs, and content. Three providers are supported natively.

## AWS S3

```env
STORAGE_PROVIDER=aws

# Credentials
S3_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE
S3_SECRET_ACCESS_KEY=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY

# Region and endpoint
S3_REGION=us-east-1
S3_ENDPOINT=  # Leave empty for AWS S3

# Bucket names
META_BUCKET=flow-like-meta
CONTENT_BUCKET=flow-like-content
LOGS_BUCKET=flow-like-logs
```

### IAM permissions

The credentials need the following permissions on your buckets:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "s3:GetObject",
        "s3:PutObject",
        "s3:DeleteObject",
        "s3:ListBucket"
      ],
      "Resource": [
        "arn:aws:s3:::flow-like-*",
        "arn:aws:s3:::flow-like-*/*"
      ]
    }
  ]
}
```

## Cloudflare R2

R2 is S3-compatible and uses the AWS provider:

```env
STORAGE_PROVIDER=aws

# R2 credentials (from R2 API tokens)
S3_ACCESS_KEY_ID=your-r2-access-key-id
S3_SECRET_ACCESS_KEY=your-r2-secret-access-key

# R2 endpoint (replace with your account ID)
S3_ENDPOINT=https://<account-id>.r2.cloudflarestorage.com
S3_REGION=auto
S3_USE_PATH_STYLE=true

# Bucket names
META_BUCKET=flow-like-meta
CONTENT_BUCKET=flow-like-content
LOGS_BUCKET=flow-like-logs
```

## MinIO (Self-hosted)

For local development or air-gapped environments:

```env
STORAGE_PROVIDER=aws

# MinIO credentials
S3_ACCESS_KEY_ID=minioadmin
S3_SECRET_ACCESS_KEY=minioadmin

# MinIO endpoint
S3_ENDPOINT=http://minio:9000
S3_REGION=us-east-1
S3_USE_PATH_STYLE=true

# Bucket names
META_BUCKET=flow-like-meta
CONTENT_BUCKET=flow-like-content
LOGS_BUCKET=flow-like-logs
```

To add MinIO to your Docker Compose stack, add this service:

```yaml
services:
  minio:
    image: minio/minio
    command: server /data --console-address ":9001"
    ports:
      - "9000:9000"
      - "9001:9001"  # Console
    environment:
      MINIO_ROOT_USER: minioadmin
      MINIO_ROOT_PASSWORD: minioadmin
    volumes:
      - minio_data:/data
    networks:
      - flowlike

volumes:
  minio_data:
```

## Azure Blob Storage

```env
STORAGE_PROVIDER=azure

# Azure credentials
AZURE_STORAGE_ACCOUNT=yourstorageaccount
AZURE_STORAGE_ACCESS_KEY=your-access-key

# Container names (Azure calls them containers, not buckets)
META_BUCKET=flow-like-meta
CONTENT_BUCKET=flow-like-content
LOGS_BUCKET=flow-like-logs
```

### Creating containers

```bash
az storage container create --name flow-like-meta --account-name yourstorageaccount
az storage container create --name flow-like-content --account-name yourstorageaccount
az storage container create --name flow-like-logs --account-name yourstorageaccount
```

## Google Cloud Storage

```env
STORAGE_PROVIDER=gcp

# GCP project
GCS_PROJECT_ID=your-project-id

# Service account JSON (base64 encoded or raw)
GOOGLE_APPLICATION_CREDENTIALS_JSON={"type":"service_account","project_id":"..."}

# Bucket names
META_BUCKET=flow-like-meta
CONTENT_BUCKET=flow-like-content
LOGS_BUCKET=flow-like-logs
```

### Service account permissions

The service account needs the `Storage Object Admin` role on your buckets:

```bash
gsutil iam ch serviceAccount:your-sa@project.iam.gserviceaccount.com:objectAdmin gs://flow-like-meta
gsutil iam ch serviceAccount:your-sa@project.iam.gserviceaccount.com:objectAdmin gs://flow-like-content
gsutil iam ch serviceAccount:your-sa@project.iam.gserviceaccount.com:objectAdmin gs://flow-like-logs
```

## Path-style URLs

Some S3-compatible providers (MinIO, R2) require path-style URLs:

```env
S3_USE_PATH_STYLE=true
```

This changes requests from:
- Virtual-hosted style: `https://bucket.endpoint.com/key`
- Path style: `https://endpoint.com/bucket/key`
