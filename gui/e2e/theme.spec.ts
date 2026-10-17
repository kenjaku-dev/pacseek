// AE theme system e2e — boot resolution, UI toggle, persistence, system
// follow. The bootstrap script in index.html resolves BEFORE first paint,
// so assertions read documentElement.dataset.theme (what the pixels show).
import { expect, test } from "@playwright/test";

async function dataTheme(page: import("@playwright/test").Page): Promise<string | undefined> {
  return page.evaluate(() => document.documentElement.dataset.theme);
}

test("no pin + dark OS → dark (brand default) through the toggle UI", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "dark" });
  await page.goto("/");
  expect(await dataTheme(page)).toBe("dark");
  await page.getByLabel("Theme").getByRole("radio", { name: "Light theme" }).click();
  expect(await dataTheme(page)).toBe("light");
  // aria state tracks the selection
  await expect(page.getByRole("radio", { name: "Light theme" })).toHaveAttribute(
    "aria-checked",
    "true",
  );
});

test("system follow: light OS flips the scheme without a pin", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "light" });
  await page.goto("/");
  expect(await dataTheme(page)).toBe("light");
  // re-pin system after a light pin: back to following the OS
  await page.getByLabel("Theme").getByRole("radio", { name: "Follow system theme" }).click();
  expect(await dataTheme(page)).toBe("light");
});

test("a stored pin survives reload and beats the OS", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "light" });
  await page.addInitScript(() => window.localStorage.setItem("pakmap-theme", "dark"));
  await page.goto("/");
  expect(await dataTheme(page)).toBe("dark");
  await page.reload();
  expect(await dataTheme(page)).toBe("dark");
});

test("light theme paints the light palette (token check)", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "light" });
  await page.goto("/");
  const bg = await page.evaluate(() =>
    getComputedStyle(document.documentElement)
      .getPropertyValue("--pk-background")
      .trim(),
  );
  // the light background slot itself (computed colours animate for 160ms on
  // switches, so assert the token — the e2e caught that exact race)
  expect(bg).toBe("#f1f5f9");
});
