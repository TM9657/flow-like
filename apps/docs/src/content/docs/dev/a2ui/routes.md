---
title: Routes API
description: Programmatically manage app routes and navigation
sidebar:
  order: 4
---

Routes in Flow-Like map URL paths to pages or events. This guide covers the technical implementation of the routing system.

## Route Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Request Flow                              │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│   User Request                                               │
│       │                                                      │
│       ▼                                                      │
│   ┌─────────────────┐                                       │
│   │  Route Matcher  │ ← Priority & Path Matching             │
│   └────────┬────────┘                                       │
│            │                                                 │
│            ▼                                                 │
│   ┌─────────────────────────────────────────────┐           │
│   │              Target Resolution               │           │
│   ├─────────────────────┬───────────────────────┤           │
│   │   targetType: page  │  targetType: event    │           │
│   │         │           │         │             │           │
│   │         ▼           │         ▼             │           │
│   │   ┌─────────┐       │   ┌─────────┐         │           │
│   │   │  Page   │       │   │  Event  │         │           │
│   │   │ Render  │       │   │ Handler │         │           │
│   │   └─────────┘       │   └─────────┘         │           │
│   └─────────────────────┴───────────────────────┘           │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Route Interface

### IAppRoute

The full route object stored in the system:

```typescript
interface IAppRoute {
  id: string;                    // Unique route identifier
  appId: string;                 // Parent app ID
  path: string;                  // URL path (e.g., "/dashboard")
  targetType: RouteTargetType;   // "page" | "event"
  pageId?: string;               // Target page ID (if targetType is "page")
  boardId?: string;              // Associated board/flow ID
  pageVersion?: Version;         // Specific page version to use
  eventId?: string;              // Target event ID (if targetType is "event")
  isDefault: boolean;            // Is this the default/home route
  priority: number;              // Route matching priority (higher = first)
  label?: string;                // Display label for navigation
  icon?: string;                 // Icon identifier for navigation
  createdAt: string;             // ISO timestamp
  updatedAt: string;             // ISO timestamp
}
```

### RouteTargetType

```typescript
type RouteTargetType = "page" | "event";
```

| Value | Description |
|-------|-------------|
| `"page"` | Route renders an A2UI page |
| `"event"` | Route triggers a flow event |

## Route State API

### Interface

```typescript
interface IAppRouteState {
  // Query routes
  getRoutes(appId: string): Promise<IAppRoute[]>;
  getRouteByPath(appId: string, path: string): Promise<IAppRoute | null>;
  getDefaultRoute(appId: string): Promise<IAppRoute | null>;

  // Mutate routes
  createRoute(appId: string, route: CreateAppRoute): Promise<IAppRoute>;
  updateRoute(appId: string, routeId: string, route: UpdateAppRoute): Promise<IAppRoute>;
  deleteRoute(appId: string, routeId: string): Promise<void>;
}
```

### CreateAppRoute

When creating a route, you don't need to specify `id`, `appId`, `createdAt`, or `updatedAt`:

```typescript
interface CreateAppRoute {
  path: string;                  // Required: URL path
  targetType: RouteTargetType;   // Required: "page" or "event"
  pageId?: string;               // Required if targetType is "page"
  boardId?: string;              // Optional: associated board
  pageVersion?: Version;         // Optional: pin to specific version
  eventId?: string;              // Required if targetType is "event"
  isDefault?: boolean;           // Default: false
  priority?: number;             // Default: 0
  label?: string;                // Optional: display label
  icon?: string;                 // Optional: icon name
}
```

### UpdateAppRoute

All fields are optional when updating:

```typescript
interface UpdateAppRoute {
  path?: string;
  targetType?: RouteTargetType;
  pageId?: string;
  boardId?: string;
  pageVersion?: Version;
  eventId?: string;
  isDefault?: boolean;
  priority?: number;
  label?: string;
  icon?: string;
}
```

## Usage Examples

### Get All Routes

```typescript
const routes = await backend.routeState.getRoutes(appId);
console.log(`App has ${routes.length} routes`);
```

### Get Route by Path

```typescript
const route = await backend.routeState.getRouteByPath(appId, "/dashboard");
if (route) {
  console.log(`Dashboard route points to ${route.targetType}: ${route.pageId || route.eventId}`);
}
```

### Get Default Route

```typescript
const homeRoute = await backend.routeState.getDefaultRoute(appId);
if (homeRoute) {
  // Redirect to home page
  navigateTo(homeRoute.path);
}
```

### Create a Page Route

```typescript
const newRoute = await backend.routeState.createRoute(appId, {
  path: "/reports",
  targetType: "page",
  pageId: "page_abc123",
  boardId: "board_xyz789",
  label: "Reports",
  icon: "chart",
});
```

### Create an Event Route

```typescript
const webhookRoute = await backend.routeState.createRoute(appId, {
  path: "/api/webhook",
  targetType: "event",
  eventId: "event_webhook_handler",
  priority: 10, // Higher priority for API routes
});
```

### Update a Route

```typescript
// Make a route the default
await backend.routeState.updateRoute(appId, routeId, {
  isDefault: true,
});

// Change route target
await backend.routeState.updateRoute(appId, routeId, {
  targetType: "page",
  pageId: "new_page_id",
  eventId: undefined, // Clear the event ID
});
```

### Delete a Route

```typescript
await backend.routeState.deleteRoute(appId, routeId);
```

## Route Resolution

### Path Matching

Routes are matched in this order:

1. **Exact match** - Path matches exactly
2. **Priority order** - Higher `priority` value checked first
3. **Creation order** - Earlier routes checked first

### Default Route

The default route (`isDefault: true`) is used when:

- User navigates to the app root (`/`)
- No other route matches the requested path
- Only one route can be default per app

### Version Pinning

For page routes, you can pin to a specific page version:

```typescript
{
  path: "/stable-dashboard",
  targetType: "page",
  pageId: "dashboard",
  pageVersion: {
    major: 1,
    minor: 0,
    patch: 0,
    type: "stable"
  }
}
```

Version types:

| Type | Description |
|------|-------------|
| `"draft"` | Work in progress |
| `"preview"` | Testing version |
| `"stable"` | Production-ready |
| `"archived"` | Deprecated |

## Navigation

### From Flows

Use navigation nodes to redirect users:

```
[Navigate Node]
├── Route Path: "/success"
└── Mode: "replace" | "push"
```

### From Components

A2UI components can trigger navigation:

```json
{
  "type": "Button",
  "props": {
    "label": "Go to Reports"
  },
  "actions": {
    "onClick": {
      "type": "navigate",
      "path": "/reports"
    }
  }
}
```

### Programmatic Navigation

From frontend code:

```typescript
// Using the router
router.push(`/use?app=${appId}&route=/dashboard`);

// With route state
const route = await backend.routeState.getRouteByPath(appId, targetPath);
if (route) {
  // Handle the route target
}
```

## Error Handling

### Route Not Found

When no route matches:

```typescript
const route = await backend.routeState.getRouteByPath(appId, requestedPath);
if (!route) {
  // Fall back to default route
  const defaultRoute = await backend.routeState.getDefaultRoute(appId);
  if (!defaultRoute) {
    // Show 404 or app landing
  }
}
```

### Duplicate Path Prevention

Creating a route with a duplicate path will fail:

```typescript
try {
  await backend.routeState.createRoute(appId, {
    path: "/existing-path", // Already exists
    targetType: "page",
    pageId: "some-page",
  });
} catch (error) {
  // Handle duplicate path error
}
```

## Best Practices

### API Routes

For event-based API routes:

- Use `/api/` prefix for clarity
- Set higher priority to avoid page route conflicts
- Consider authentication requirements

```typescript
{
  path: "/api/data",
  targetType: "event",
  eventId: "data_handler",
  priority: 100, // High priority
}
```

### Page Routes

For page routes:

- Use descriptive, lowercase paths
- Group related pages under common prefixes
- Set appropriate labels for navigation UI

```typescript
{
  path: "/settings/profile",
  targetType: "page",
  pageId: "profile_settings_page",
  label: "Profile Settings",
  icon: "user",
}
```

### Default Route

Always have a default route:

```typescript
{
  path: "/",
  targetType: "page",
  pageId: "home_page",
  isDefault: true,
  label: "Home",
  icon: "home",
}
```
