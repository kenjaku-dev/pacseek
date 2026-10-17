// axe-core smoke over the key states (docs/gui-architecture.md testing table):
// empty / results done / detail open / error card / shortcuts dialog.
// Violations = test failure, so regressions block the local gate.
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

async function scan(page: import("@playwright/test").Page): Promise<void> {
  const { violations } = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
    .analyze();
  const summary = violations.map(
    (v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ") + ' <' + n.html.slice(0, 160) + '>').join(", ")}`,
  );
  expect(summary, `axe violations:\n${summary.join("\n")}`).toEqual([]);
}

/**
 * Pin the theme explicitly: Playwright's default color scheme is LIGHT, so an
 * unpinned boot would silently flip every existing scan to the light palette.
 * The brand default (dark) keeps the original semantics; light runs get their
 * own dedicated scans below (AE contrast for the derived scheme).
 */
async function pinTheme(
  page: import("@playwright/test").Page,
  theme: "light" | "dark",
): Promise<void> {
  await page.addInitScript(
    (t) => window.localStorage.setItem("pakmap-theme", t),
    theme,
  );
}

test("empty state passes axe (wcag 2.1 AA)", async ({ page }) => {
  await pinTheme(page, "dark");
  await page.goto("/");
  await expect(page.getByRole("banner")).toBeVisible();
  await scan(page);
});

test("done state passes axe", async ({ page }) => {
  await pinTheme(page, "dark");
  await page.goto("/");
  await page.getByLabel("Search packages").fill("firefox");
  await page.getByLabel("Search packages").press("Enter");
  await expect(page.getByRole("status")).toHaveText(/Found 6/, { timeout: 3_000 });
  await scan(page);
});

test("detail pane open passes axe", async ({ page }) => {
  await pinTheme(page, "dark");
  await page.goto("/");
  await page.getByLabel("Search packages").fill("firefox");
  await page.getByLabel("Search packages").press("Enter");
  await expect(page.getByRole("status")).toHaveText(/Found 6/, { timeout: 3_000 });
  await page.getByRole("option").filter({ hasText: "Mozilla Firefox web browser" }).first().click();
  await expect(page.getByRole("complementary", { name: "Package detail" })).toBeVisible();
  await scan(page);
});

test("error card state passes axe", async ({ page }) => {
  await pinTheme(page, "dark");
  await page.goto("/");
  await page.getByLabel("Search packages").fill("err-db-locked");
  await page.getByLabel("Search packages").press("Enter");
  await expect(page.getByRole("alert")).toContainText("repo_search_failed", { timeout: 2_000 });
  await scan(page);
});

test("shortcuts dialog (modal) passes axe", async ({ page }) => {
  await pinTheme(page, "dark");
  await page.goto("/");
  await page.keyboard.press("?");
  await expect(page.getByRole("dialog", { name: "Keyboard shortcuts" })).toBeVisible();
  await scan(page);
});

test("updates tab passes axe", async ({ page }) => {
  await pinTheme(page, "dark");
  await page.goto("/");
  await page.getByRole("tab", { name: "Updates" }).click();
  await expect(page.getByRole("heading", { name: "Updates" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Upgrade all (4)" })).toBeVisible();
  await scan(page);
});

test("news tab passes axe", async ({ page }) => {
  await pinTheme(page, "dark");
  await page.goto("/");
  await page.getByRole("tab", { name: "News" }).click();
  await expect(page.getByRole("alert")).toContainText("Manual intervention required");
  await scan(page);
});

test("confirm dialog (a11y modal) passes axe", async ({ page }) => {
  await pinTheme(page, "dark");
  await page.goto("/");
  await page.getByRole("tab", { name: "Updates" }).click();
  await page.getByRole("button", { name: "Refresh dbs" }).click();
  await expect(page.getByRole("dialog", { name: "Refresh databases" })).toBeVisible();
  await scan(page);
});

test("ops banner states pass axe (running and failed)", async ({ page }) => {
  await pinTheme(page, "dark");
  await page.goto("/");
  await page.getByRole("tab", { name: "Updates" }).click();
  await page.getByRole("button", { name: "Update err-auth-sample" }).click();
  await page
    .getByRole("dialog", { name: "Update err-auth-sample" })
    .getByRole("button", { name: "Update package" })
    .click();
  const result = page.getByRole("status", { name: "Operation result" });
  await expect(result).toContainText("auth_dismissed", { timeout: 5_000 });
  await scan(page);
});

// ---------------------------------------------------------------------------
// AE: the light scheme is an independently designed palette, not an inversion
// — it gets its own conformance scans for the two core reading states.
// ---------------------------------------------------------------------------

test("light theme: empty state passes axe", async ({ page }) => {
  await pinTheme(page, "light");
  await page.goto("/");
  await expect(page.getByRole("banner")).toBeVisible();
  await scan(page);
});

test("light theme: done state (list + badges) passes axe", async ({ page }) => {
  await pinTheme(page, "light");
  await page.goto("/");
  await page.getByLabel("Search packages").fill("firefox");
  await page.getByLabel("Search packages").press("Enter");
  await expect(page.getByRole("status")).toHaveText(/Found 6/, { timeout: 3_000 });
  await page.getByRole("option").filter({ hasText: "Mozilla Firefox web browser" }).first().click();
  await expect(page.getByRole("complementary", { name: "Package detail" })).toBeVisible();
  await scan(page);
});

test("light theme: ops banner failure passes axe", async ({ page }) => {
  await pinTheme(page, "light");
  await page.goto("/");
  await page.getByRole("tab", { name: "Updates" }).click();
  await page.getByRole("button", { name: "Update err-auth-sample" }).click();
  await page
    .getByRole("dialog", { name: "Update err-auth-sample" })
    .getByRole("button", { name: "Update package" })
    .click();
  const result = page.getByRole("status", { name: "Operation result" });
  await expect(result).toContainText("auth_dismissed", { timeout: 5_000 });
  await scan(page);
});
