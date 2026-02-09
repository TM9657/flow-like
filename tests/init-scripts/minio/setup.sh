#!/bin/bash
set -e

echo "Setting up MinIO alias..."
mc alias set myminio http://minio:9000 flowlike flowlike_test

echo "Creating test buckets..."
mc mb myminio/flowlike-test --ignore-existing
mc mb myminio/flowlike-delta --ignore-existing
mc mb myminio/flowlike-parquet --ignore-existing
mc mb myminio/flowlike-content --ignore-existing
mc mb myminio/flowlike-meta --ignore-existing
mc mb myminio/flowlike-logs --ignore-existing

echo "Creating directory structure for scoped credential tests..."
echo "" | mc pipe myminio/flowlike-content/apps/test-app-123/.keep
echo "" | mc pipe myminio/flowlike-content/users/user-abc/apps/test-app-123/.keep
echo "" | mc pipe myminio/flowlike-content/tmp/user/user-abc/apps/test-app-123/.keep
echo "" | mc pipe myminio/flowlike-content/tmp/global/apps/test-app-123/.keep
echo "" | mc pipe myminio/flowlike-logs/runs/test-app-123/.keep

echo "Creating scoped user policy..."
cat > /tmp/scoped-policy.json << 'EOF'
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": ["s3:ListBucket"],
      "Resource": ["arn:aws:s3:::flowlike-content", "arn:aws:s3:::flowlike-meta"],
      "Condition": {
        "StringLike": {
          "s3:prefix": [
            "apps/test-app-123/*",
            "users/user-abc/apps/test-app-123/*",
            "tmp/user/user-abc/apps/test-app-123/*",
            "tmp/global/apps/test-app-123/*"
          ]
        }
      }
    },
    {
      "Effect": "Allow",
      "Action": ["s3:GetObject", "s3:PutObject", "s3:DeleteObject"],
      "Resource": [
        "arn:aws:s3:::flowlike-content/apps/test-app-123/*",
        "arn:aws:s3:::flowlike-content/users/user-abc/apps/test-app-123/*",
        "arn:aws:s3:::flowlike-content/tmp/user/user-abc/apps/test-app-123/*",
        "arn:aws:s3:::flowlike-content/tmp/global/apps/test-app-123/*"
      ]
    },
    {
      "Effect": "Allow",
      "Action": ["s3:GetObject", "s3:PutObject"],
      "Resource": ["arn:aws:s3:::flowlike-logs/runs/test-app-123/*"]
    }
  ]
}
EOF

echo "Creating deny-all policy..."
cat > /tmp/deny-policy.json << 'EOF'
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Deny",
      "Action": ["s3:*"],
      "Resource": ["arn:aws:s3:::*"]
    }
  ]
}
EOF

echo "Adding policies and users..."
mc admin policy create myminio scoped-test-policy /tmp/scoped-policy.json || echo "Policy may already exist"
mc admin user add myminio scoped-user scoped-user-secret || echo "User may already exist"
mc admin policy attach myminio scoped-test-policy --user scoped-user || echo "Policy may already be attached"

mc admin policy create myminio deny-all-policy /tmp/deny-policy.json || echo "Policy may already exist"
mc admin user add myminio denied-user denied-user-secret || echo "User may already exist"
mc admin policy attach myminio deny-all-policy --user denied-user || echo "Policy may already be attached"

mc anonymous set download myminio/flowlike-test

echo "MinIO setup complete with scoped credential users!"
