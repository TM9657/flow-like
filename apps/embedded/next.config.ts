"use client";

/** @type {import('next').NextConfig} */
const nextConfig = {
  output: "export",
  pageExtensions: ["js", "jsx", "md", "mdx", "ts", "tsx"],
  images: {
    unoptimized: true,
  },
  staticPageGenerationTimeout: 120,
  missingSuspenseWithCSRBailout: false,
  experimental: {
    missingSuspenseWithCSRBailout: false,
  },
  devIndicators: {
    appIsrStatus: false,
  },
};

export default nextConfig;