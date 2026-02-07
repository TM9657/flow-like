import type { Config } from "tailwindcss";

const config = {
	content: [
		"./components/**/*.{ts,tsx}",
		"./hooks/**/*.{ts,tsx}",
		"./lib/**/*.{ts,tsx}",
		"./state/**/*.{ts,tsx}",
		"./pages/**/*.{ts,tsx}",
		"./src/**/*.{ts,tsx}",
		"./app/**/*.{ts,tsx}",
	],
	theme: {
		extend: {},
	},
	plugins: [],
} satisfies Config;

export default config;
