import { fileURLToPath } from "node:url";
import tailwind from "@tailwindcss/postcss";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

const root = fileURLToPath(new URL("./", import.meta.url));
const repo = fileURLToPath(new URL("../../", import.meta.url));
export default defineConfig({
	root,
	define: {
		"process.env": "{}",
		__FORECAST_MONACO_PATH__: JSON.stringify(
			`/@fs${repo}node_modules/monaco-editor/min/vs`,
		),
	},
	plugins: [react()],
	resolve: { dedupe: ["react", "react-dom", "@tanstack/react-query"] },
	css: { postcss: { plugins: [tailwind({ base: root })] } },
	server: {
		host: "127.0.0.1",
		port: 4344,
		strictPort: true,
		hmr: false,
		watch: { ignored: ["**/target/**", "**/.next/**", "**/dist/**"] },
		fs: { allow: [repo] },
	},
});
