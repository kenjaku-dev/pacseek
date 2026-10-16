import type { NextConfig } from "next";

// GitHub Pages project site needs a base path; local/prod root deploys leave it empty.
const basePath = process.env.NEXT_PUBLIC_BASE_PATH ?? "";

// Version lives in src/lib/version.ts — regenerate via `pnpm sync-version`
// (reads ../Cargo.toml). See README.md#conventions.

const nextConfig: NextConfig = {
  output: "export",
  images: { unoptimized: true },
  basePath,
  assetPrefix: basePath || undefined,
};

export default nextConfig;
