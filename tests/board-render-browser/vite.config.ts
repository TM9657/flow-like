import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import tailwind from "@tailwindcss/postcss";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
	root: fileURLToPath(new URL("./", import.meta.url)),
	define: { "process.env": "{}" },
	plugins: [
		react(),
		{
			name: "board-benchmark-font-license",
			apply: "build",
			generateBundle() {
				this.emitFile({
					type: "asset",
					fileName: "OFL-Inter.txt",
					source: readFileSync(
						new URL("./assets/OFL-Inter.txt", import.meta.url),
						"utf8",
					),
				});
			},
		},
	],
	resolve: { dedupe: ["react", "react-dom"] },
	css: { postcss: { plugins: [tailwind()] } },
	build: { outDir: "../../tmp/board-render-benchmark/site", emptyOutDir: true },
	server: {
		host: "127.0.0.1",
		port: 4336,
		strictPort: true,
		fs: { allow: [fileURLToPath(new URL("../../", import.meta.url))] },
	},
	preview: { host: "127.0.0.1", port: 4336, strictPort: true },
});
