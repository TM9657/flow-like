import react from "@astrojs/react";
import starlight from "@astrojs/starlight";
import tailwindcss from "@tailwindcss/vite";

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
			social: [
				{
					icon: "discord",
					label: "Discord",
					href: "https://discord.gg/KTWMrS2",
				},
				{
					icon: "github",
					label: "GitHub",
					href: "https://github.com/TM9657/flow-like",
				},
				{ icon: "x.com", label: "X.com", href: "https://x.com/greatco_de" },
				{
					icon: "linkedin",
					label: "LinkedIn",
					href: "https://linkedin.com/company/greatco-de",
				},
			],
			lastUpdated: true,
			sidebar: [
				{
					label: "Guides",
					autogenerate: { directory: "guides" },
				},
				{
					label: "Contributing",
					autogenerate: { directory: "contributing" },
				},
				{
					label: "Nodes",
					autogenerate: { directory: "nodes" },
				},
				{
					label: "Reference",
					autogenerate: { directory: "reference" },
				},
			],
		}),
	],
	vite: {
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
		define: {
			// stub out `process.env` so next/dist/client code can import it without blowing up
			"process.env": {},
			// if any code reads process.env.NODE_ENV, you can explicitly set it:
			"process.env.NODE_ENV": JSON.stringify(
				process.env.NODE_ENV || "production",
			),
		},
		plugins: [
			{
				name: "ignore-css-imports-ssr",
				enforce: "pre",
				load(id, { ssr }) {
					if (ssr && id.endsWith(".css")) {
						// pretend it was an empty module
						return { code: "" };
					}
				},
			},
			tailwindcss(),
		],
	},
});
