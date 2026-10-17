// AD operations e2e — tabs, updates screen, confirm → progress → done banner
// flow, failure classification surfacing, news screen, cancel semantics copy.
// All against the scripted mock gateway (same envelope shapes as the backend).
import { expect, test, type Page } from "@playwright/test";

async function gotoUpdates(page: Page): Promise<void> {
  await page.goto("/");
  await page.getByRole("tab", { name: "Updates" }).click();
  await expect(page.getByRole("heading", { name: "Updates" })).toBeVisible();
}

test("tabs render and switch (a11y tablist semantics)", async ({ page }) => {
  await page.goto("/");
  const tablist = page.getByRole("tablist", { name: "Areas" });
  await expect(tablist).toBeVisible();
  await expect(tablist.getByRole("tab", { name: "Search" })).toHaveAttribute("aria-selected", "true");
  await page.getByRole("tab", { name: "Updates" }).click();
  await expect(page.getByRole("tab", { name: "Updates" })).toHaveAttribute("aria-selected", "true");
  await expect(page.getByRole("tabpanel", { name: "Updates" })).toBeVisible();
  await page.getByRole("tab", { name: "News" }).click();
  await expect(page.getByRole("tabpanel", { name: "News" })).toBeVisible();
  // hotkey back to search
  await page.keyboard.press("1");
  await expect(page.getByRole("tabpanel", { name: "Search" })).toBeVisible();
});

test("updates screen renders both legs with counts and tools", async ({ page }) => {
  await gotoUpdates(page);
  await expect(page.getByText("Checking for updates…")).toBeHidden();
  await expect(page.getByRole("button", { name: "Upgrade all (4)" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Refresh dbs" })).toBeVisible();
  await expect(page.getByRole("button", { name: /Remove orphans \(2\)/ })).toBeVisible();
  await expect(page.getByRole("list", { name: "Official repository updates" })).toContainText("firefox");
  await expect(page.getByRole("list", { name: "Official repository updates" })).toContainText(
    "139.0-1 → 140.0-2",
  );
  await expect(page.getByRole("list", { name: "AUR updates" })).toContainText("firefox-bin");
});

test("update-all: confirm dialog, progress banner, terminal done, dismiss clears", async ({
  page,
}) => {
  await gotoUpdates(page);
  await page.getByRole("button", { name: "Upgrade all (4)" }).click();
  const dialog = page.getByRole("dialog", { name: "Full system upgrade" });
  await expect(dialog).toBeVisible();
  await expect(dialog).toContainText("never killed mid-flight");
  // Cancel keeps the world unchanged
  await dialog.getByRole("button", { name: "Cancel" }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);

  // run for real
  await page.getByRole("button", { name: "Upgrade all (4)" }).click();
  await dialog.getByRole("button", { name: "Upgrade system" }).click();
  const running = page.getByRole("status", { name: "Operation progress" });
  await expect(running).toBeVisible();
  await expect(running).toContainText("Full system upgrade");
  const result = page.getByRole("status", { name: "Operation result" });
  await expect(result).toContainText("done", { timeout: 5_000 });
  // journal toggle reveals the scripted lines
  await page.getByRole("button", { name: "Expand operation journal" }).click();
  await expect(page.getByTestId("op-journal")).toContainText("prepare: scripted line 1");
  await page.getByRole("button", { name: "Dismiss operation result" }).click();
  await expect(result).toHaveCount(0);
});

test("cancel: phase-boundary semantics reach the terminal state", async ({ page }) => {
  await gotoUpdates(page);
  await page.getByRole("button", { name: "Refresh dbs" }).click();
  await page.getByRole("dialog", { name: "Refresh databases" }).getByRole("button", { name: "Refresh", exact: true }).click();
  const progress = page.getByRole("status", { name: "Operation progress" });
  await expect(progress).toBeVisible();
  await progress.getByRole("button", { name: "Cancel" }).click();
  const result = page.getByRole("status", { name: "Operation result" });
  // the backend (and mock) terminate with the classified canceled envelope
  await expect(result).toContainText("op_canceled", { timeout: 5_000 });
  await expect(result).toContainText("never killed mid-flight");
  await page.getByRole("button", { name: "Dismiss operation result" }).click();
  await expect(result).toHaveCount(0);
});

test("classified failure: error code + hint surface in the banner", async ({ page }) => {
  await gotoUpdates(page);
  await page.getByRole("button", { name: "Update err-auth-sample" }).click();
  await page
    .getByRole("dialog", { name: "Update err-auth-sample" })
    .getByRole("button", { name: "Update package" })
    .click();
  const result = page.getByRole("status", { name: "Operation result" });
  await expect(result).toContainText("auth_dismissed", { timeout: 5_000 });
  await expect(result).toContainText("policykit denied the request");
});

test("news screen: intervention banner, action badge, copy-link action", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("tab", { name: "News" }).click();
  await expect(page.getByRole("alert")).toContainText("Manual intervention required");
  await expect(page.getByText("[ACTION REQUIRED]")).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Copy link to news item: Regular update round" }),
  ).toBeVisible();
});

test("detail pane actions: remove flow opens the confirm with risk copy", async ({ page }) => {
  await page.goto("/");
  await page.getByLabel("Search packages").fill("firefox");
  await page.getByLabel("Search packages").press("Enter");
  await expect(page.getByRole("status").first()).toContainText(/Found/, { timeout: 3_000 });
  // installed repo row (firefox) — open detail
  await page.getByRole("option").filter({ hasText: "Mozilla Firefox web browser" }).first().click();
  const pane = page.getByRole("complementary", { name: "Package detail" });
  await expect(pane).toBeVisible();
  await expect(pane.getByRole("button", { name: "Remove" })).toBeVisible();
  await pane.getByRole("button", { name: "Remove" }).click();
  const dialog = page.getByRole("dialog", { name: "Remove firefox" });
  await expect(dialog).toContainText("pkexec pacman -Rs --noconfirm");
  await expect(dialog).toContainText("your settings files are kept");
  await dialog.getByRole("button", { name: "Remove", exact: true }).click();
  await expect(page.getByRole("status", { name: "Operation progress" })).toBeVisible();
});
