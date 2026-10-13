import { tmpdir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import tailwind from "@tailwindcss/postcss";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

const root = fileURLToPath(new URL("./", import.meta.url));
const repo = fileURLToPath(new URL("../../", import.meta.url));

export default defineConfig({
	root,
	cacheDir: resolve(tmpdir(), "flow-like-chat-rendering-vite"),
	define: { "process.env": "{}" },
	plugins: [
		react(),
		{
			name: "chat-preview-boundaries",
			enforce: "pre",
			resolveId(id, importer) {
				if (!importer || !id.startsWith(".")) return;
				const path = resolve(dirname(importer), id);
				if (
					path === `${repo}packages/ui` ||
					path === `${repo}packages/ui/index` ||
					path === `${repo}packages/ui/components/ui` ||
					path === `${repo}packages/ui/components/ui/index`
				)
					return `${root}ui-exports.ts`;
				if (path === `${repo}packages/ui/lib`)
					return `${repo}packages/ui/lib/utils.ts`;
				if (path === `${repo}packages/ui/state/backend-state`)
					return `${root}backend-fixture.ts`;
				// This preview exercises read-only rendering; editing is never mounted.
				if (path === `${repo}packages/ui/components/ui/text-editor-editable`)
					return "\0chat-preview:editable";
			},
			load(id) {
				if (id === "\0chat-preview:editable")
					return "export const TextEditorEditable = () => null;";
			},
		},
	],
	resolve: { dedupe: ["react", "react-dom", "@tanstack/react-query"] },
	css: { postcss: { plugins: [tailwind({ base: root })] } },
	server: {
		host: "127.0.0.1",
		port: 4341,
		strictPort: true,
		hmr: false,
		watch: { ignored: ["**/target/**", "**/.next/**", "**/dist/**"] },
		fs: { allow: [repo] },
	},
	optimizeDeps: {
		include: ["react", "react-dom/client", "react-oidc-context"],
	},
});
