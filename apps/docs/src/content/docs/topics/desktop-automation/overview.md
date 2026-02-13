---
title: Desktop Automation & RPA
description: Automate desktop applications with screen capture, OCR, and input simulation
sidebar:
  order: 1
---

Flow-Like brings **RPA (Robotic Process Automation)** capabilities to your desktop, allowing you to automate interactions with any application—even legacy systems without APIs.

## Capabilities Overview

| Feature | Description | Status |
|---------|-------------|--------|
| **Screen Capture** | Take screenshots of full screen or regions | ✅ Available |
| **AI OCR** | Extract text from images using vision AI | ✅ Available |
| **Barcode/QR Reading** | Decode QR codes and barcodes | ✅ Available |
| **Keyboard Automation** | Type text and press key combinations | 🔜 Coming |
| **Mouse Automation** | Click, drag, scroll at positions | 🔜 Coming |
| **Window Management** | Focus windows, launch apps | 🔜 Coming |
| **Visual Element Finding** | Locate UI elements by image template | 🔜 Coming |
| **Workflow Recording** | Record actions to generate automation | 🔜 Coming |

## Current Capabilities

### AI-Powered OCR

Extract text from any screen capture or document using vision-capable AI models:

```
Screenshot / Image
    │
    ▼
AI Extract Document (Vision Model)
    │
    ▼
Extracted Text (Markdown format)
```

**Supported formats:**
- Screenshots (PNG, JPG)
- PDFs (rendered to images)
- Scanned documents
- Photos of documents

**Example: Read text from screen region**
```
Capture Screen Region (x, y, width, height)
    │
    ▼
AI Extract Document
├── model: GPT-4 Vision / Claude Vision
└── prompt: "Extract all visible text"
    │
    ▼
Extracted Text ──▶ Process / Store
```

### Barcode & QR Code Reading

Decode barcodes and QR codes from images:

```
Read Barcodes (image)
    │
    ▼
Array of detected codes:
├── type: QR_CODE
├── data: "https://example.com/product/123"
└── position: { x, y, width, height }
```

**Supported formats:**
- QR Code
- PDF417
- Code 128
- Code 39
- EAN-13/8
- UPC-A/E
- DataMatrix
- Aztec

**Example: Process shipping labels**
```
For Each image in shipping_labels
    │
    ▼
Read Barcodes (image)
    │
    ▼
Extract tracking number ──▶ Add to database
```

### QR Code Generation

Create QR codes programmatically:

```
Write QR Code
├── data: "https://myapp.com/order/12345"
├── size: 256
└── format: PNG
    │
    ▼
QR Code Image ──▶ Save / Display / Email
```

### IP Camera Integration

Capture frames from network cameras for monitoring:

```
Grab Camera Frame (mjpeg_url)
    │
    ▼
Image ──▶ AI Analysis / Store / Alert
```

**Use cases:**
- Inventory monitoring
- Security alerts
- Production line inspection

## Planned RPA Capabilities

### Mouse Automation

Control mouse movements and clicks:

| Node | Description |
|------|-------------|
| **Click At Position** | Click at specific x,y coordinates |
| **Click Template** | Click on visually matched element |
| **Double Click** | Double-click at position |
| **Mouse Drag** | Drag from point A to point B |
| **Scroll** | Scroll up/down at position |

**Example: Automate form filling**
```
Click At Position (100, 200)  ──▶ Focus name field
    │
    ▼
Type Text ("John Doe")
    │
    ▼
Click At Position (100, 250)  ──▶ Focus email field
    │
    ▼
Type Text ("john@example.com")
    │
    ▼
Click Template (submit_button.png)  ──▶ Click submit
```

### Keyboard Automation

Simulate keyboard input:

| Node | Description |
|------|-------------|
| **Type Text** | Type a string of text |
| **Key Press** | Press a single key with modifiers |
| **Key Combination** | Press shortcuts (Ctrl+C, Alt+Tab) |

**Example: Copy data from legacy app**
```
Focus Window ("Legacy CRM")
    │
    ▼
Key Press (Ctrl+A)  ──▶ Select all
    │
    ▼
Key Press (Ctrl+C)  ──▶ Copy
    │
    ▼
Get Clipboard ──▶ Process copied data
```

### Window Management

Control application windows:

| Node | Description |
|------|-------------|
| **Focus Window** | Bring window to front by title |
| **Launch App** | Start an application |
| **Minimize/Maximize** | Control window state |
| **Close Window** | Close application window |

### Visual Element Finding

Locate UI elements using template matching:

```
Find Template (button_image.png)
    │
    ▼
Position: { x: 450, y: 320, confidence: 0.95 }
    │
    ▼
Click At Position (450, 320)
```

**With fallback:**
```
Click Template
├── template: submit_button.png
├── fallback_position: (500, 400)
├── timeout: 5000ms
└── confidence_threshold: 0.8
```

### Workflow Recording

Record your actions to generate automation:

1. **Start Recording** – Begin capture mode
2. **Perform Actions** – Click, type, navigate normally
3. **Stop Recording** – End capture
4. **Review Generated Flow** – Edit the automation
5. **Run** – Execute the recorded workflow

The recorder captures:
- Mouse clicks with screenshots
- Keyboard input
- Window focus changes
- Timing between actions

## Permission Requirements

Desktop automation requires system permissions:

### macOS

| Permission | Purpose | How to Enable |
|------------|---------|---------------|
| **Accessibility** | Mouse/keyboard control | System Preferences → Security & Privacy → Accessibility |
| **Screen Recording** | Take screenshots | System Preferences → Security & Privacy → Screen Recording |

Flow-Like requests these permissions automatically when needed.

### Windows

- Run as Administrator for some applications
- No special permissions typically required

### Linux

- X11 input extension for keyboard/mouse
- Screenshot permissions may vary by desktop environment

## Use Cases

### Legacy System Integration

Automate data entry into systems without APIs:

```
For Each record in new_records
    │
    ▼
Focus Window ("Legacy ERP")
    │
    ▼
Click Template (new_record_button.png)
    │
    ├──▶ Type in Field 1 (record.name)
    ├──▶ Type in Field 2 (record.value)
    └──▶ Click Submit
```

### Data Extraction

Pull data from desktop applications:

```
Focus Window ("Financial Software")
    │
    ▼
Navigate to Reports
    │
    ▼
Screenshot Report Area
    │
    ▼
AI Extract (table data)
    │
    ▼
Parse and Store in Database
```

### Automated Testing

Test desktop applications:

```
Launch App ("MyApp.exe")
    │
    ▼
Wait for Window ("Main Window")
    │
    ▼
Click "Login" button
    │
    ├──▶ Type username
    ├──▶ Type password
    └──▶ Click Submit
    │
    ▼
Verify: Window title contains "Dashboard"
```

### Document Processing Pipeline

Combine RPA with document processing:

```
Watch Folder (/incoming)
    │
    ▼
For Each new file
    │
    ├── PDF? ──▶ AI Extract Document ──▶ Process
    ├── Image? ──▶ AI OCR ──▶ Process
    └── Scanned? ──▶ AI Vision Extract ──▶ Process
```

## Best Practices

### 1. Use Image Templates Wisely

- Capture unique UI elements
- Avoid templates with dynamic content
- Include enough context for reliable matching
- Test at different screen resolutions

### 2. Add Wait/Retry Logic

```
Retry (3 times, 1s delay)
    │
    ▼
Find Template (loading_complete.png)
    │
    ▼
Continue with automation
```

### 3. Handle Errors Gracefully

```
Try
    │
    ▼
Click Template (button.png)
    │
    └── Catch: Template not found
            │
            ▼
        Take Screenshot ──▶ Log error ──▶ Alert user
```

### 4. Run Unattended Carefully

- Test thoroughly in attended mode first
- Add checkpoints and logging
- Implement timeout limits
- Have recovery procedures

### 5. Respect Rate Limits

- Add delays between actions (200-500ms)
- Don't overwhelm target applications
- Simulate human-like interaction speeds

## Combining with AI

RPA becomes powerful when combined with AI:

### Intelligent Data Extraction

```
Screenshot Application
    │
    ▼
AI Vision: "Find all customer records in this screenshot"
    │
    ▼
Structured Data: [{name, email, status}, ...]
    │
    ▼
Store in Database
```

### Decision-Based Automation

```
Screenshot Current State
    │
    ▼
AI Analysis: "What is the application state?"
    │
    ▼
Branch based on AI response:
├── "Login screen" ──▶ Perform login
├── "Dashboard" ──▶ Navigate to reports
└── "Error dialog" ──▶ Handle error
```

### Natural Language Instructions

```
Chat Event: "Download the latest sales report"
    │
    ▼
AI Plans actions:
├── Open reporting application
├── Navigate to sales reports
├── Select latest report
└── Click download
    │
    ▼
Execute each action
```

## FAQ

### Can I automate any application?
Yes, if it has a visible UI, you can automate it. Some applications with custom rendering may be harder to work with.

### Does it work in the background?
Currently, the target window needs to be visible. Background automation is planned.

### How reliable is template matching?
Very reliable when done correctly. Use unique, stable UI elements and set appropriate confidence thresholds.

### Can I run multiple automations simultaneously?
One automation can run at a time on a single machine. For parallel execution, use multiple machines.

### Is it secure?
Yes—automations run locally on your machine. No screenshots or data are sent anywhere unless you explicitly configure it.

## Next Steps

- **[Document Processing](/topics/document-processing/overview/)** – Extract data from documents
- **[API Integrations](/topics/api-integrations/overview/)** – When APIs are available
- **[GenAI](/topics/genai/overview/)** – AI-powered analysis
- **[Building Internal Tools](/topics/internal-tools/overview/)** – Create control UIs for automations
