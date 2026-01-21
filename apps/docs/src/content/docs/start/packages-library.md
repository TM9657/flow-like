---
title: Package Library
description: Manage your installed WASM packages
sidebar:
  order: 29
---

The Package Library shows all WASM packages installed on your system. From here you can update packages, uninstall ones you no longer need, and publish your own packages.

## Accessing Your Library

Navigate to **Library → Packages** in the sidebar to view your installed packages.

## Installed Packages View

Each installed package shows:

- **Package name and icon**
- **Current version** installed
- **Update available** indicator (if a newer version exists)
- **Action buttons** for update and uninstall

## Updating Packages

### Update Single Package

1. Find the package with an update available
2. Click the **Update** button
3. Wait for the new version to download

### Update All Packages

If you have multiple packages with updates available:

1. Click the **Update All** button at the top
2. All outdated packages will update sequentially

:::tip
Keep packages updated to get the latest features, bug fixes, and security patches.
:::

## Uninstalling Packages

1. Find the package you want to remove
2. Click the **Uninstall** button
3. Confirm the removal

After uninstalling:
- The package's nodes will no longer appear in the workflow editor
- Any workflows using those nodes will show errors
- You can reinstall the package anytime from the Store

## Publishing Your Own Package

Click **Publish Package** to open the publishing wizard:

### Step 1: Upload

- Click **Select File** or drag your `.wasm` file
- The file will be validated automatically
- Maximum file size: 50MB

### Step 2: Manifest

Review and edit your package metadata:

| Field | Description |
|-------|-------------|
| **ID** | Unique identifier (e.g., `my-org.my-package`) |
| **Name** | Display name |
| **Version** | Semantic version (e.g., `1.0.0`) |
| **Description** | What your package does |
| **Author** | Your name or organization |
| **License** | Open source license (MIT, Apache-2.0, etc.) |
| **Repository** | GitHub repository URL (optional) |
| **Homepage** | Project website (optional) |
| **Keywords** | Search terms to help users find your package |

### Step 3: Permissions

Configure what system access your package needs:

- **File System Access**: Can read/write files
- **Network Access**: Can make HTTP requests
- **Environment Access**: Can read environment variables
- **Process Access**: Can spawn processes (rarely needed)

:::caution
Only request permissions your package actually uses. Packages requesting unnecessary permissions may be declined during review.
:::

### Step 4: Review & Submit

- Review all information
- Accept the terms of service
- Click **Publish Package**

After submission, your package enters the review queue. You'll be notified when it's approved or if changes are needed.

## Package Status

Your published packages can have these statuses:

| Status | Meaning |
|--------|---------|
| **Pending Review** | Waiting for admin approval |
| **Active** | Approved and available in the store |
| **Changes Requested** | Reviewer requested modifications |
| **Deprecated** | Still works but not recommended |
| **Disabled** | Removed from the registry |

## Local Development

For testing packages during development:

1. Build your WASM module locally
2. In Library → Packages, use **Install from file**
3. Select your local `.wasm` file
4. Test the package in your workflows
5. When ready, publish to the registry

This lets you iterate quickly without going through review for every change.

## Troubleshooting

### Upload Fails

- Ensure the file is a valid `.wasm` module
- Check file size is under 50MB
- Verify your internet connection

### Manifest Errors

- Package ID must be in `org.package-name` format
- Version must follow semver (e.g., `1.0.0`)
- Description is required

### Review Takes Too Long

- Most packages are reviewed within 48-72 hours
- Complex packages may take longer
- Contact support if waiting more than a week

## Next Steps

- [Package Store](/start/packages-store) - Browse and install packages
- [Creating WASM Nodes](/dev/wasm-nodes/overview) - Build custom nodes
- [Governance & Approval](/dev/wasm-nodes/registry#governance--approval-process) - Learn about the review process
