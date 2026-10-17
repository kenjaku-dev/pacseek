// Vitest — unit tests only (src); Playwright owns e2e/*.spec.ts.
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["src/**/*.{test,spec}.{ts,tsx}"],
  },
});
