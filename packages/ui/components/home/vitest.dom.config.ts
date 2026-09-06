import { defineConfig } from "vitest/config";

export default defineConfig({
	esbuild: { jsx: "automatic" },
	test: {
		environment: "happy-dom",
		include: ["packages/ui/components/home/*.dom.test.tsx"],
		pool: "forks",
		maxWorkers: 1,
		testTimeout: 15_000,
	},
});
