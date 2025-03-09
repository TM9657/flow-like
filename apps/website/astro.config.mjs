import markdoc from "@astrojs/markdoc";
import react from "@astrojs/react";
import sitemap from "@astrojs/sitemap";
import tailwind from "@astrojs/tailwind";
import playformCompress from "@playform/compress";
import robotsTxt from "astro-robots-txt";
import { defineConfig } from "astro/config";

// https://astro.build/config
export default defineConfig({
	site: "https://flow-like.com",
	integrations: [
		react(),
		tailwind({
			applyBaseStyles: false,
		}),
		markdoc(),
		robotsTxt(),
		sitemap(),
		playformCompress(),
	],
	output: "static",
	markdown: {
		shikiConfig: {
			themes: {
				light: "min-light",
				dark: "nord",
			},
			wrap: true,
		},
	},
});
