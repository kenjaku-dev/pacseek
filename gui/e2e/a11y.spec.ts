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

test("empty state passes axe (wcag 2.1 AA)", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("banner")).toBeVisible();
  await scan(page);
});

test("done state passes axe", async ({ page }) => {
  await page.goto("/");
  await page.getByLabel("Search packages").fill("firefox");
  await page.getByLabel("Search packages").press("Enter");
  await expect(page.getByRole("status")).toHaveText(/Found 6/, { timeout: 3_000 });
  await scan(page);
});

test("detail pane open passes axe", async ({ page }) => {
  await page.goto("/");
  await page.getByLabel("Search packages").fill("firefox");
  await page.getByLabel("Search packages").press("Enter");
  await expect(page.getByRole("status")).toHaveText(/Found 6/, { timeout: 3_000 });
  await page.getByRole("option").filter({ hasText: "Mozilla Firefox web browser" }).first().click();
  await expect(page.getByRole("complementary", { name: "Package detail" })).toBeVisible();
  await scan(page);
});

test("error card state passes axe", async ({ page }) => {
  await page.goto("/");
  await page.getByLabel("Search packages").fill("err-db-locked");
  await page.getByLabel("Search packages").press("Enter");
  await expect(page.getByRole("alert")).toContainText("repo_search_failed", { timeout: 2_000 });
  await scan(page);
});

test("shortcuts dialog (modal) passes axe", async ({ page }) => {
  await page.goto("/");
  await page.keyboard.press("?");
  await expect(page.getByRole("dialog", { name: "Keyboard shortcuts" })).toBeVisible();
  await scan(page);
});
