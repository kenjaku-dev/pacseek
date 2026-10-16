import type { MetadataRoute } from "next";

// Required for output: "export" (static export).
export const dynamic = "force-static";

// Single-page site; the base path is part of the absolute URL (see pages.yml).
export default function sitemap(): MetadataRoute.Sitemap {
  return [
    {
      url: "https://kenjaku-dev.github.io/pakmap/",
      lastModified: new Date(),
      changeFrequency: "monthly",
      priority: 1,
    },
  ];
}
