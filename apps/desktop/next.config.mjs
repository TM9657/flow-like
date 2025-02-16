"use client"
import path from "path"
import { withSentryConfig } from "@sentry/nextjs";
/** @type {import('next').NextConfig} */
const nextConfig = {
    output: 'export',
    pageExtensions: ['js', 'jsx', 'md', 'mdx', 'ts', 'tsx'],
    images:{
      unoptimized: true
    },
    staticPageGenerationTimeout: 120,
    missingSuspenseWithCSRBailout: false,
    experimental: {
      missingSuspenseWithCSRBailout: false,
    },
    devIndicators: {
      appIsrStatus: false,
      
    },
  }

export default nextConfig
// export default  withSentryConfig(nextConfig, {
//   org: "good-code",
//   project: "flow-like-desktop",

//   // An auth token is required for uploading source maps.
//   authToken: process.env.SENTRY_AUTH_TOKEN,

//   silent: false, // Can be used to suppress logs
// });
