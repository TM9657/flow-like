import react from "@astrojs/react";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "astro/config";

import mdx from "@astrojs/mdx";

import sitemap from "@astrojs/sitemap";

// https://astro.build/config
export default defineConfig({
	site: "https://flow-like.com",
	integrations: [
		// markdoc(),
		// robotsTxt(),
		sitemap(),
		// playformCompress(),
		react(),
		mdx({
			syntaxHighlight: "shiki",
			shikiConfig: { theme: "dracula" },
			remarkRehype: { footnoteLabel: "Footnotes" },
			gfm: false,
		}),
		sitemap(),
	],
	vite: {
		define: {
			"process.env": {},
		},
		ssr: {
			noExternal: [
				"katex",
				"rehype-katex",
				"@tm9657/flow-like-ui",
				"lodash-es",
				"@platejs/math",
				"react-lite-youtube-embed",
				"react-tweet",
			],
		},
		plugins: [tailwindcss()],
	},
	output: "static",
	markdown: {
		syntaxHighlight: "shiki",
		shikiConfig: {
			themes: {
				light: "min-light",
				dark: "dracula",
			},
			wrap: true,
		},
	},
});
