// Playwright smoke — pakmap-gui search experience against the Vite dev server
// in mock-IPC mode. The real backend can't be driven from a plain browser, so
// the UI runs against the scripted gateway (src/lib/mockIpc.ts) through the
// same src/lib/ipc.ts seam; fast timers via VITE_PAKMAP_MOCK_FAST=1.
import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  timeout: 20_000,
  fullyParallel: true,
  retries: process.env.CI ? 1 : 0,
  reporter: [["list"]],
  use: {
    baseURL: "http://127.0.0.1:5199",
    trace: "retain-on-failure",
  },
  webServer: {
    command: "pnpm exec vite --port 5199 --host 127.0.0.1 --strictPort",
    port: 5199,
    reuseExistingServer: !process.env.CI,
    env: {
      VITE_PAKMAP_MOCK_IPC: "1",
      VITE_PAKMAP_MOCK_FAST: "1",
    },
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
});
