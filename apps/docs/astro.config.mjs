import react from "@astrojs/react";
import starlight from "@astrojs/starlight";
import tailwind from "@astrojs/tailwind";
// @ts-check
import { defineConfig, passthroughImageService } from "astro/config";
// https://astro.build/config
export default defineConfig({
	site: "https://docs.flow-like.com",
	output: "static",
	image: {
		service: passthroughImageService(),
	},
	integrations: [
		react(),
		starlight({
			title: "Flow-Like",
			favicon: "/ico-light.svg",
			description:
				"Flow-Like is a visual programming language for creating very fast and efficient workflows and automations.",
			head: [
				{
					tag: "link",
					attrs: {
						rel: "icon",
						href: "/ico.ico",
						sizes: "32x32",
					},
				},
			],
			editLink: {
				baseUrl: "https://github.com/TM9657/flow-like/edit/main/apps/docs/",
			},
			logo: {
				light: "./src/assets/app-logo-light.webp",
				dark: "./src/assets/app-logo.webp",
			},
			customCss: ["./src/tailwind.css"],
			social: {
				github: "https://github.com/TM9657/flow-like",
				"x.com": "https://x.com/tm9657",
				linkedin: "https://linkedin.com/company/tm9657",
				discord: "https://discord.gg/KTWMrS2",
			},
			lastUpdated: true,
			sidebar: [
				{
					label: "Guides",
					autogenerate: { directory: "guides" },
				},
				{
					label: "Nodes",
					autogenerate: { directory: "nodes" },
				},
				{
					label: "Contributing",
					autogenerate: { directory: "contributing" },
				},
				{
					label: "Reference",
					autogenerate: { directory: "reference" },
				},
			],
		}),
		tailwind({
			applyBaseStyles: false,
		}),
	],
});
