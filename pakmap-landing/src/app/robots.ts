import type { MetadataRoute } from "next";

// Required for output: "export" (static export).
export const dynamic = "force-static";

// Absolute URL; the site is always deployed under https://kenjaku-dev.github.io/pakmap/
// (see .github/workflows/pages.yml) so the base path is part of the const.
export default function robots(): MetadataRoute.Robots {
  return {
    rules: { userAgent: "*", allow: "/" },
    sitemap: "https://kenjaku-dev.github.io/pakmap/sitemap.xml",
  };
}
