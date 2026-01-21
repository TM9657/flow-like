---
title: Local-Only Execution
description: Flows that require Studio to run locally on your device
sidebar:
  order: 45
---

Some flows in Flow-Like **must run locally** on your device through Flow-Like Studio. These flows cannot be executed on remote servers or in the cloud—they require direct access to your machine's resources.

## Why Some Flows Require Local Execution

Certain automation tasks need capabilities that only exist on your local machine:

- **Direct hardware access** (cameras, microphones, USB devices)
- **Desktop control** (mouse, keyboard, screen capture)
- **Local file system access** (reading/writing files on your computer)
- **Browser automation** (controlling a real browser window)
- **Compute-intensive processing** (leveraging your local GPU or CPU)

When you add a node that requires local execution to your flow, the entire board is marked as **local-only**. This ensures your automation runs reliably with full access to the resources it needs.

## Examples of Local-Only Nodes

### Robotic Process Automation (RPA)

RPA nodes automate interactions with your desktop applications:

- **Click** – Simulate mouse clicks at specific screen coordinates
- **Type** – Send keystrokes to applications
- **Screenshot** – Capture your screen or specific windows
- **Window Control** – Focus, minimize, maximize, or close windows
- **OCR (Optical Character Recognition)** – Read text from screen regions

These nodes are essential for automating legacy applications that don't have APIs.

### Browser Automation

Control a real browser on your machine:

- **Navigate** – Open URLs in a controlled browser
- **Click Element** – Click buttons, links, or other page elements
- **Fill Form** – Enter text into form fields
- **Extract Data** – Scrape content from web pages
- **Take Screenshot** – Capture page screenshots

Browser automation requires a local browser instance (Chromium) that can only run on your device.

### Local File Operations

Work directly with files on your computer:

- **Read File** – Load content from local files
- **Write File** – Save data to your file system
- **Watch Folder** – Monitor directories for changes
- **Execute Program** – Run local applications or scripts

### Hardware Integration

Access devices connected to your machine:

- **Camera Capture** – Take photos or record video
- **Microphone Input** – Record audio
- **Speaker Output** – Play sounds

### AI & Machine Learning

Some AI operations run best (or only) on local hardware:

- **Local LLM Inference** – Run language models on your GPU
- **Image Processing** – GPU-accelerated image operations
- **Speech Recognition** – On-device voice transcription

## How to Identify Local-Only Flows

In Flow-Like Studio, boards containing local-only nodes are visually marked. When you run a pre-flight check on your flow, you'll see:

- A **local execution required** indicator
- The specific nodes that require local execution

## Running Local-Only Flows

Local-only flows can be triggered in several ways:

1. **Manual execution** – Click the Run button in Studio
2. **Scheduled events** – Set up timed triggers (your computer must be on)
3. **File watchers** – Automatically trigger when files change
4. **Hotkeys** – Assign keyboard shortcuts to start your flow

:::note
For scheduled or automated local flows, Flow-Like Studio must be running on your machine. Consider enabling "Start on Login" in your system preferences.
:::

## Mixing Local and Remote Nodes

You can combine local-only nodes with standard nodes in the same flow. The entire flow will execute locally, but you can still:

- Make API calls to external services
- Store data in cloud databases
- Send notifications via email or messaging platforms

This gives you the best of both worlds—local device control with cloud connectivity.

## Best Practices

1. **Test locally first** – Always verify your RPA and browser automation flows work correctly before scheduling them
2. **Handle errors gracefully** – Desktop automation can fail if windows move or UI changes; add error handling nodes
3. **Use delays wisely** – Some applications need time to respond; add appropriate wait nodes
4. **Document your flows** – Use comment nodes to explain what each RPA step does for future maintenance
