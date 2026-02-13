---
title: Package Store
description: Browse, install, and manage community WASM packages
sidebar:
  order: 28
---

The Package Store allows you to extend Flow-Like with community-created nodes. Browse verified packages, install with a single click, and keep your packages up to date.

## Accessing the Store

Navigate to **Store → Packages** in the sidebar to open the package registry.

## Browsing Packages

### Search

Use the search bar to find packages by:
- Package name
- Description keywords
- Author name

### Filtering

- **Verified only**: Toggle to show only packages reviewed and approved by the Flow-Like team
- This is recommended for most users to ensure quality and security

### Sorting

Sort packages by:
- **Downloads**: Most popular packages first
- **Relevance**: Best match for your search query
- **Name**: Alphabetical order
- **Updated**: Recently updated packages first
- **Created**: Newest packages first

## Package Details

Click on any package card to view its details page with four tabs:

### Overview

- **Description**: What the package does
- **Author**: Who created the package
- **License**: Terms of use
- **Downloads**: How many times installed
- **Links**: GitHub repository, homepage, documentation

### Nodes

A list of all workflow nodes included in the package:
- Node names and descriptions
- Input and output pins
- Categories

### Permissions

What system access the package requires:
- **None**: Pure computation, safest option
- **Network**: Can make HTTP requests
- **File System**: Can read/write files
- **Environment**: Can access environment variables

:::tip
Prefer packages that request minimal permissions. A package should only request what it actually needs.
:::

### Versions

Version history showing:
- Version numbers
- Release dates
- Release notes (when available)

## Installing Packages

### From the Store

1. Find the package you want
2. Click on it to open the detail page
3. Click the **Install** button
4. Wait for the download to complete

The package's nodes will immediately be available in your workflow editor.

### Checking for Updates

Return to the package detail page anytime. If a newer version is available, you'll see an **Update** button instead of "Installed".

## What is a Verified Package?

Packages with the **✓ Verified** badge have been:

- Reviewed by Flow-Like maintainers
- Checked for security issues
- Tested for correct functionality
- Confirmed to follow best practices

We recommend only installing verified packages unless you trust the package author.

## Troubleshooting

### Package Won't Install

- Check your internet connection
- Try again later—the registry may be temporarily unavailable
- Ensure you have sufficient disk space

### Package Nodes Don't Appear

- Close and reopen the workflow editor
- Check the node catalog under the package's category
- Restart Flow-Like Desktop if needed

### Package Doesn't Work as Expected

- Check the package's documentation or GitHub issues
- Ensure you're using the latest version
- Report issues to the package author

## Next Steps

- [Managing Installed Packages](/start/packages-library) - Update and uninstall packages
- [Creating Custom Packages](/dev/wasm-nodes/overview) - Build your own WASM nodes
- [Publishing Packages](/dev/wasm-nodes/registry) - Share your packages with the community
