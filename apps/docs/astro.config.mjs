// @ts-check
import { defineConfig, passthroughImageService } from 'astro/config';
import starlight from '@astrojs/starlight';
import tailwind from '@astrojs/tailwind';
import react from "@astrojs/react";
// https://astro.build/config
export default defineConfig({
	site: "https://docs.flow-like.com",
	output: "static",
	image: {
		service: passthroughImageService()
	  },
	integrations: [
		react(),
		starlight({
			title: 'Flow-Like',
			editLink: {
				baseUrl: 'https://github.com/TM9657/flow-like/edit/main/apps/docs/',
			  },
			logo: {
				light: './src/assets/dark-mode.svg',
				dark: './src/assets/dark-mode.svg',
			},
			customCss: [
				'./src/tailwind.css',
			],
			social: {
				github: 'https://github.com/TM9657/flow-like',
				"x.com": "https://x.com/tm9657",
				linkedin: "https://linkedin.com/company/tm9657",
			},
			sidebar: [
				{
					label: 'Guides',
					autogenerate: { directory: 'guides' },
				},
				{
					label: 'Nodes',
					autogenerate: { directory: 'nodes' },
				},
				{
					label: 'Contributing',
					autogenerate: { directory: 'contributing' },
				},
				{
					label: 'Reference',
					autogenerate: { directory: 'reference' },
				},
			],
		}),
		tailwind({
			applyBaseStyles: false,
		  }),
	],
});
