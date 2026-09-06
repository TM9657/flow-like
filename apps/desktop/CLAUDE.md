# Desktop App (Tauri + Next.js)

## Dev Server

```bash
bun run dev         # Next.js only
bun run dev:all     # Tauri + Next.js
bunx tsc --noEmit   # Type-check
bunx biome check .  # Lint
```

## TypeScript

- Use interfaces for data structures and type definitions.
- Prefer immutable data: `const`, `readonly`.
- Use optional chaining (`?.`) and nullish coalescing (`??`).
- Follow functional programming principles where possible.

## React

- Functional components with hooks only.
- `React.FC` for components with children.
- Follow hooks rules: never call hooks conditionally.
- Keep components small and focused — split into subcomponents.
- Colocate small subcomponents in the same file.

## UI Framework

- **shadcn** components are pre-installed — import them, never recreate.
- **Tailwind CSS** for all styling — use design system tokens, not raw hex values.
- **Lucide** for icons — `import { IconName } from "lucide-react"`.

## Performance

- `useMemo` / `useCallback` for expensive computations and stable references.
- Proper `useEffect` dependency arrays — no missing or extraneous deps.
- `useState` for local state, avoid unnecessary re-renders.

<!-- BEGIN:nextjs-agent-rules -->

# This is NOT the Next.js you know

This version has breaking changes — APIs, conventions, and file structure may all differ from your training data. Read the relevant guide in `node_modules/next/dist/docs/` (resolved from this file's directory; in monorepos the `next` package may not be visible from the repo root) before writing any code. Heed deprecation notices.

This block is written and re-added by `next dev` — verify at `node_modules/next/dist/server/lib/generate-agent-files.js`. Removing it from a diff only re-creates the uncommitted change; committing it with your work keeps the tree clean.

<!-- END:nextjs-agent-rules -->
