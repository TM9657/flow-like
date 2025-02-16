
import type { Config } from "tailwindcss"
import preset from "@tm9657/flow-like-ui/tailwind.config"

const config = {
  presets: [preset],
  content: [
    './src/**/*.{astro,html,js,jsx,md,mdx,svelte,ts,tsx,vue}',
    '../../node_modules/@tm9657/flow-like-ui/**/*.{ts,tsx}'
  ],
} satisfies Config

export default config