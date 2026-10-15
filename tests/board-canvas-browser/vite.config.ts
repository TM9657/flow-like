import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import tailwind from "@tailwindcss/postcss";
import react from "@vitejs/plugin-react";
import { defineConfig, transformWithEsbuild } from "vite";

export default defineConfig({
	root: fileURLToPath(new URL("./", import.meta.url)),
	publicDir: "../../apps/web/public",
	define: { "process.env": "{}" },
	plugins: [
		react(),
		{
			name: "harness-layer-node-type",
			enforce: "pre",
			resolveId(id) {
				if (id.endsWith("/layer-inner-node"))
					return "\0harness-layer-node-type";
			},
			async load(id) {
				if (id !== "\0harness-layer-node-type") return;
				// Validation is the real module. Its unused rich layer editor is replaced with its exact source enum.
				const source = readFileSync(
					new URL(
						"../../packages/ui/components/flow/layer-inner-node.tsx",
						import.meta.url,
					),
					"utf8",
				);
				const declaration = source.match(
					/export enum InnerLayerNodeType\s*\{[^}]*\}/,
				)?.[0];
				if (!declaration)
					throw new Error("Cannot read production layer boundary enum");
				return transformWithEsbuild(declaration, "layer-node-type.ts");
			},
		},
	],
	resolve: { dedupe: ["react", "react-dom"] },
	css: { postcss: { plugins: [tailwind()] } },
	build: { outDir: "../../tmp/board-canvas-browser/site", emptyOutDir: true },
	server: {
		host: "127.0.0.1",
		port: 4338,
		strictPort: true,
		fs: { allow: [fileURLToPath(new URL("../../", import.meta.url))] },
	},
	preview: { host: "127.0.0.1", port: 4338, strictPort: true },
});
