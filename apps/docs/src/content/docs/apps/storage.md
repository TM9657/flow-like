---
title: Storage
description: Upload and manage files for your apps
sidebar:
  order: 30
---

Every Flow-Like app has its own **file storage** for the assets your flows read and write. Two views manage it:

- **Storage**: upload and manage shared files and directories for the app. Select files from your local machine or drag and drop them directly.
- **User Storage**: browse and search your own private, per-user files within the app.

:::tip[Looking for databases?]
Relational tables and the semantic layer built on top of them now live in **[Data Studio](/apps/data-studio/)**.
:::

**User Storage** appears in the App navigation when [Developer
Mode](/start/developer-mode/) is enabled. Access permissions still apply to both
views. For queued uploads and cached tables in an online App, see
[Offline access](/apps/offline-access/).

## Upload Directory
Here you can upload files and entire folders to make them part of your app. For online apps, the files are stored in the cloud and synchronized across all devices where you are logged in with your Flow-Like account:

![A screenshot of Flow-Like Desktop showing the upload directory of an app](../../../assets/AppStorage.webp)

## View and edit Word documents

Select a `.docx` file in **Storage** to open **Flows Word**, the document viewer
and editor. It loads when you open a document and supports files up to 50 MB.

If your role allows writing files, choose **Edit document** to make changes, then
**Save**. Saving updates the selected file in its current folder. Closing the
editor with unsaved changes asks whether to discard them.

DOCX editing is experimental. Complex formatting or unsupported document features
may change when saved, so review the saved document in Microsoft Word before
sharing it. Flows Word is available in the shared **Storage** tab; legacy `.doc`
files are not supported.
