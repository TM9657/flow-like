---
title: Routes
description: Map URL paths to pages and events in your app
sidebar:
  order: 42
---

Routes define how users navigate your app. Each route maps a URL path (like `/dashboard` or `/settings`) to either a page or an event.

## What are Routes?

A route is a connection between a URL and content:

```
URL Path           →    Target
────────────────────────────────────
/                  →    Dashboard Page (default)
/reports           →    Reports Page
/settings          →    Settings Event
/products/:id      →    Product Detail Page
```

When a user visits your app with a specific path, Flow-Like shows the corresponding page or triggers the associated event.

## Why Routes?

Routes let you:

- **Create multi-page apps** - Build apps with distinct sections
- **Deep link** - Share direct links to specific content
- **Control navigation** - Define which pages are accessible
- **Set a home page** - Choose what users see first

## Managing Routes

### Access Route Settings

1. Open your app
2. Click the **gear icon** (⚙️) to open settings
3. Go to **Pages & Routes**
4. Switch to the **Routes** tab

### The Routes Panel

```
┌─────────────────────────────────────────────────────────────────────┐
│  Routes                                              [+ Add Route]  │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │  /                               [Default]                    │   │
│  │  Page: Dashboard                           🔘 [🗑]            │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │  /reports                                                     │   │
│  │  Page: Monthly Reports                     ○  [🗑]            │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │  /api/webhook                                                 │   │
│  │  Event: Webhook Handler                    ○  [🗑]            │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

## Creating a Route

1. Click **Add Route**
2. Fill in the route details:

| Field | Description |
|-------|-------------|
| **Path** | The URL path (e.g., `/dashboard`) |
| **Target Type** | Page or Event |
| **Target** | Which page or event to show |
| **Default** | Whether this is the home page |

3. Click **Create Route**

### Route Paths

Paths start with `/` and can include:

| Pattern | Example | Matches |
|---------|---------|---------|
| Static | `/about` | Exactly `/about` |
| Nested | `/settings/profile` | Exactly `/settings/profile` |

## Route Targets

A route can point to two types of targets:

### Page Target

Shows a visual page when the route is accessed:

```
Route: /dashboard
Target Type: Page
Target: Dashboard Page

→ User visits /dashboard
→ Dashboard Page is rendered
```

### Event Target

Triggers a flow event when the route is accessed:

```
Route: /api/process
Target Type: Event
Target: Process Handler

→ User visits /api/process
→ Process Handler event runs
→ Response is returned
```

This is useful for:
- API endpoints
- Webhooks
- Server-side processing

## Default Route

The default route is shown when users visit your app without a specific path.

To set a default route:

1. Find the route in the list
2. Toggle the **Default** switch
3. Only one route can be default at a time

:::tip[Home Page]
Always set a default route so users don't see a blank screen when opening your app.
:::

## Route Priority

When multiple routes could match a path, Flow-Like uses priority:

1. **Exact matches** first
2. Then **priority number** (higher = checked first)
3. Finally **creation order**

You can adjust priority by reordering routes in the settings.

## Navigation Between Routes

### User Navigation

Users navigate between routes using:

- **Links** - Clickable text or buttons
- **Navigation menus** - Built-in navigation components
- **Direct URL** - Typing or sharing links

### Programmatic Navigation

From your flows, you can:

- **Redirect** - Send users to a different route
- **Navigate** - Change the current route
- **Open in new tab** - Launch a route in a new window

## Best Practices

### Path Naming

| ✅ Good | ❌ Avoid |
|---------|----------|
| `/dashboard` | `/page1` |
| `/settings/profile` | `/mySettings` |
| `/products` | `/product_list` |

Use lowercase, hyphens for spaces, and descriptive names.

### Organizing Routes

For complex apps, group related routes:

```
/                      → Home
/dashboard             → Dashboard
/dashboard/stats       → Detailed Stats
/settings              → Settings Overview
/settings/profile      → User Profile
/settings/billing      → Billing Settings
```

### Error Handling

Consider creating:

- **404 page** - For unknown routes
- **Error page** - For failures
- **Loading page** - For slow content

## Route Examples

### Basic App

```
/           → Landing Page (default)
/features   → Features Page
/pricing    → Pricing Page
/contact    → Contact Form Page
```

### Dashboard App

```
/                → Dashboard (default)
/analytics       → Analytics Page
/reports         → Reports Page
/settings        → Settings Page
```

### API App

```
/              → Documentation Page (default)
/api/submit    → Submit Handler (event)
/api/status    → Status Handler (event)
/webhook       → Webhook Handler (event)
```

## What's Next?

- **[Pages](/apps/pages/)** - Design visual pages for your routes
- **[Events](/apps/events/)** - Create event handlers for API routes
- **[Sharing](/apps/share/)** - Share your app with others
