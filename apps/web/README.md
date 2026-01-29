# Flow-Like Web

This is the web version of Flow-Like, a Next.js application that runs in the browser without Tauri.

## Key Differences from Desktop

- **No Tauri**: Uses standard HTTP APIs instead of Tauri invoke commands
- **Authentication**: Uses standard web OIDC flow without deep linking
- **Backend**: WebBackend uses HTTP fetch instead of Tauri's native calls
- **File Operations**: Limited to browser capabilities (no native file system access)

## Getting Started

1. Install dependencies:
```bash
bun install
```

2. Copy the example environment file:
```bash
cp .env.example .env.local
```

3. Configure your environment variables in `.env.local`

4. Run the development server:
```bash
bun run dev
```

The app will be available at [http://localhost:3001](http://localhost:3001).

## Architecture

- `components/web-provider.tsx` - Web-compatible backend provider (replaces TauriProvider)
- `components/auth-provider.tsx` - Standard web OIDC authentication (replaces Tauri deep linking)
- `lib/api.ts` - HTTP API client using fetch (replaces Tauri invoke)

## Pages

The following pages from the desktop app are included:
- `/` - Home page with swimlanes
- `/flow` - Flow editor
- `/library` - Library management
- `/settings` - Settings pages

Note: Some features that rely on native capabilities (like local file access, system tray, etc.) are not available in the web version.

## TODO

The current implementation uses empty state placeholders for most backend functionality. To make it fully functional, you need to:

1. Implement HTTP-based state providers for:
   - AppState
   - BitState
   - BoardState
   - UserState
   - StorageState
   - etc.

2. Create API endpoints that match the desktop app's functionality

3. Handle authentication properly with your OIDC provider

4. Implement file upload/download using browser APIs

5. Remove or stub out Tauri-specific functionality in copied pages
