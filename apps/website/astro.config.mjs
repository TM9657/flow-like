import { defineConfig } from 'astro/config';
import react from "@astrojs/react";
import tailwind from "@astrojs/tailwind";
import markdoc from "@astrojs/markdoc";
import robotsTxt from "astro-robots-txt";
import playformCompress from "@playform/compress";
import sitemap from "@astrojs/sitemap";

// https://astro.build/config
export default defineConfig({
  site: 'https://flow-like.com',
  integrations: [react(), tailwind({
    applyBaseStyles: false
  }), markdoc(), robotsTxt(), sitemap(), playformCompress()],
  output: "static",
  markdown: {
    shikiConfig: {
      themes: {
        light: 'min-light',
        dark: 'nord',
      },
      wrap: true,
    },
  }
});
