---
title: Package Registry
description: Publishing, installing, and managing WASM packages
sidebar:
  order: 27
---

Flow-Like provides a central package registry for discovering, sharing, and installing WASM nodes created by the community.

## Registry Overview

The package registry allows you to:

- **Browse** community-created packages
- **Search** by name, description, or keywords
- **Install** packages with a single click
- **Publish** your own packages for others to use
- **Update** installed packages when new versions are available

## Finding Packages

### In the Desktop App

Navigate to **Store → Packages** to browse the registry:

- **Search**: Type in the search bar to find packages by name or keyword
- **Filter**: Toggle "Verified only" to show only reviewed packages
- **Sort**: Order by downloads, relevance, name, or date

### Package Details

Click on any package to see:

- **Overview**: Description, author, license, and links
- **Nodes**: List of all nodes included in the package
- **Permissions**: What capabilities the package requires
- **Versions**: Version history with release notes

## Installing Packages

### From the Store

1. Navigate to **Store → Packages**
2. Find and click on the package you want
3. Click the **Install** button
4. The package will be downloaded and made available immediately

### Local Installation

You can also install packages from local `.wasm` files:

1. Navigate to **Library → Packages**
2. Click **Install from file**
3. Select your `.wasm` file

## Managing Installed Packages

Access your installed packages at **Library → Packages**:

| Action | Description |
|--------|-------------|
| **Update** | Install the latest version of a package |
| **Update All** | Update all packages with available updates |
| **Uninstall** | Remove a package from your system |

## Publishing Packages

### Prerequisites

Before publishing, ensure your package:

1. Has a valid `manifest.json` (see [Manifest Reference](/dev/wasm-nodes/manifest))
2. Compiles to a valid WASM module
3. Follows the naming convention: `your-org.package-name`
4. Includes a description and keywords

### Publishing Process

1. Navigate to **Library → Packages → Publish**
2. **Step 1 - Upload**: Select your compiled `.wasm` file
3. **Step 2 - Manifest**: Review and edit package metadata
4. **Step 3 - Permissions**: Configure required capabilities
5. **Step 4 - Review**: Verify all information and submit

After submission, your package enters the **review queue**.

## Understanding Package Status

| Status | Badge | Description |
|--------|-------|-------------|
| **Pending Review** | 🟡 | Awaiting admin review |
| **Active** | 🟢 | Approved and available |
| **Deprecated** | ⚠️ | Still available but not recommended |
| **Disabled** | 🔴 | Removed from the registry |

### Verified Packages

Packages marked with a **✓ Verified** badge have been:

- Reviewed by the Flow-Like team
- Checked for security issues
- Tested for compatibility
- Confirmed to follow best practices

---

## Governance & Approval Process

All packages submitted to the public registry must go through an approval process to ensure quality and security.

### Why Review is Required

WASM packages can execute arbitrary code with the permissions they request. The review process:

- **Protects users** from malicious or buggy code
- **Ensures quality** of the ecosystem
- **Maintains compatibility** with Flow-Like updates
- **Builds trust** in the package registry

### Submission Guidelines

Before submitting a package for review:

1. **Test thoroughly** - Ensure your package works correctly
2. **Minimize permissions** - Only request what you actually need
3. **Document clearly** - Include helpful descriptions and examples
4. **Follow naming conventions** - Use `org.package-name` format
5. **Version semantically** - Follow [semver](https://semver.org/) for versioning

### The Review Process

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Package    │     │   Admin      │     │   Package    │
│   Submitted  │────▶│   Review     │────▶│   Decision   │
└──────────────┘     └──────────────┘     └──────────────┘
       │                    │                    │
       │                    ▼                    ▼
       │            ┌──────────────┐     ┌──────────────┐
       │            │   Security   │     │  ✓ Approved  │
       │            │   Check      │     │  ✗ Declined  │
       │            └──────────────┘     │  💬 Feedback │
       │                                 └──────────────┘
       ▼
┌──────────────────────────────────────────────────────┐
│                  Pending Review                       │
│  • Package visible with "pending" status             │
│  • Cannot be installed by users yet                  │
└──────────────────────────────────────────────────────┘
```

### What Reviewers Check

| Area | What We Review |
|------|----------------|
| **Security** | Requested permissions match actual usage; no malicious patterns |
| **Quality** | Code compiles correctly; nodes function as described |
| **Metadata** | Name, description, keywords are accurate and helpful |
| **Compatibility** | Works with current Flow-Like version |
| **License** | License is valid and permits redistribution |

### Review Outcomes

After review, your package will receive one of these outcomes:

#### ✓ Approved

Your package is published to the registry:
- Status changes to **Active**
- Users can install it immediately
- You can publish updates (which also require review)

#### ✗ Declined

The package did not meet requirements:
- You'll receive feedback explaining why
- Common reasons:
  - Security concerns with permissions
  - Package doesn't work as described
  - Missing or incorrect metadata
- You can fix issues and resubmit

#### 💬 Changes Requested

Minor changes needed before approval:
- Reviewer comments explain what to fix
- Update your package and submit a new version
- The new version enters the review queue

### Review Timeline

- Most packages are reviewed within **48-72 hours**
- Complex packages may take longer
- You can check status in **Admin → Packages** (if you have admin access)

### Appealing Decisions

If you disagree with a review decision:

1. Read the feedback carefully
2. If you believe there's an error, open a GitHub issue
3. Provide context and evidence supporting your case
4. A different reviewer will evaluate the appeal

### Administrator Access

Users with the `ManagePackages` permission can:

- View all pending packages
- Review package contents and metadata
- Approve or decline submissions
- Add comments and feedback
- Disable problematic packages

Admin access is granted to trusted community members. If you're interested in helping review packages, contact the Flow-Like team.

---

## Best Practices

### For Package Authors

1. **Start small** - Begin with a single, well-tested node
2. **Request minimal permissions** - Only what you truly need
3. **Include examples** - Help users understand how to use your nodes
4. **Maintain compatibility** - Test against Flow-Like updates
5. **Respond to feedback** - Address reviewer comments promptly

### For Users

1. **Check verification status** - Prefer verified packages
2. **Review permissions** - Understand what access a package needs
3. **Keep packages updated** - Install security and bug fixes
4. **Report issues** - Help maintainers improve packages

## Local Development

For development, you can load packages locally without going through the registry:

1. Build your WASM module locally
2. Use **Library → Packages → Install from file**
3. Test in your workflows
4. Once ready, publish to the registry

This allows rapid iteration during development while maintaining quality for public releases.

## Package Permissions Reference

When reviewing or installing packages, understand these permission levels:

| Permission | Risk Level | Description |
|------------|------------|-------------|
| **None** | Safe | Pure computation, no external access |
| **Network** | Medium | Can make HTTP requests |
| **File System** | High | Can read/write files |
| **Environment** | Medium | Can access environment variables |
| **Process** | Critical | Can spawn processes (rarely granted) |

Packages should request the minimum permissions needed for their functionality.
