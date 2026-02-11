import react from "@astrojs/react";
import tailwindcss from "@tailwindcss/vite";
import compressor from "astro-compressor";
import { defineConfig } from "astro/config";

import mdx from "@astrojs/mdx";

import sitemap from "@astrojs/sitemap";

// https://astro.build/config
export default defineConfig({
	site: "https://flow-like.com",
	i18n: {
		defaultLocale: "en",
		locales: ["en", "de", "es", "fr"],
		routing: {
			prefixDefaultLocale: false,
		},
	},
	integrations: [
		// markdoc(),
		// robotsTxt(),
		sitemap(),
		// playformCompress(),
		react(),
		mdx({
			syntaxHighlight: "shiki",
			shikiConfig: {
				theme: "dracula",
				wrap: true,
			},
			remarkRehype: { footnoteLabel: "Footnotes" },
			gfm: true,
		}),
		(await import("@playform/compress")).default(),
		compressor(),
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
