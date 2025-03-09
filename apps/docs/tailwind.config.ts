import starlightPlugin from "@astrojs/starlight-tailwind";
import preset from "@tm9657/flow-like-ui/tailwind.config";
import type { Config } from "tailwindcss";
import colors from "tailwindcss/colors";

const config = {
	presets: [preset],
	content: [
		"./src/**/*.{astro,html,js,jsx,md,mdx,svelte,ts,tsx,vue}",
		"../../node_modules/@tm9657/flow-like-ui/**/*.{ts,tsx}",
	],
	plugins: [...preset.plugins, starlightPlugin()],
	theme: {
		...preset.theme,
		extend: {
			...preset.theme.extend,
			colors: {
				...preset.theme.extend.colors,
				accent: "hsl(var(--primary))",
			},
		},
	},
} satisfies Config;

export default config;
