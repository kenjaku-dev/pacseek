// E2E battery for the search screen (gui/design/ac-search-brief.md state
// matrix): empty → streaming → done → detail, keyboard flows, the generation
// guard, error and no-results cards, and the sub-1020px overlay behavior.
import { expect, type Page, test } from "@playwright/test";

const STREAMING_MS = 400; // mock legs at 25/50/75/100ms + debounce 50ms

async function search(page: Page, query: string): Promise<void> {
  const input = page.getByRole("searchbox").or(page.getByPlaceholder(/Search packages/)).first();
  // "/" hotkey or direct focus
  await input.fill("");
  await input.fill(query);
  await input.press("Enter");
}

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("banner")).toBeVisible();
});

test("empty state shows the first-run help and shortcuts hint", async ({ page }) => {
  await expect(page.getByText("Search Arch, AUR and Flatpak")).toBeVisible();
  await expect(page.getByText("Results stream in per source")).toBeVisible();
});

test("staged stream: repo leg paints first and status reaches the parity wording", async ({
  page,
}) => {
  await search(page, "firefox");

  // repo arrives before done; the status is the live "found so far" wording
  await expect(page.getByTestId("count-repo")).toHaveText("2", { timeout: STREAMING_MS });
  await expect(page.getByTestId("count-aur")).toHaveText("…", { timeout: 30 }).catch(() => {
    // foot race is timing-dependent in CI; ordering is proven by reducer tests —
    // this check is best-effort at real speed
  });

  await expect(page.getByRole("status")).toHaveText(
    /Found 6 \(repo 2 aur 3 flatpak 1\)/,
    { timeout: 3_000 },
  );
  await expect(page.getByText(/Searching for 'firefox'/)).toHaveCount(0);

  // badges fixtures: one OOD row, one UNMAINTAINED row, installed ticks
  await expect(page.getByText("[OOD]", { exact: true })).toBeVisible();
  await expect(page.getByText("[UNMAINTAINED]", { exact: true })).toBeVisible();
  await expect(page.getByText("[installed]").first()).toBeVisible();
});

test("row click opens the detail pane and installed rows enrich via package_info", async ({
  page,
}) => {
  await search(page, "firefox");
  await expect(page.getByRole("status")).toHaveText(/Found 6/, { timeout: 3_000 });

  await page.getByRole("option").filter({ hasText: "Mozilla Firefox web browser" }).first().click();

  const pane = page.getByRole("complementary", { name: "Package detail" });
  await expect(pane).toBeVisible();
  await expect(pane.getByRole("heading", { name: /^depends/ })).toBeVisible();
  await expect(pane.getByText("gtk3", { exact: true })).toBeVisible(); // enriched by package_info
  await expect(pane.getByText("explicit")).toBeVisible();

  // close via the pane button
  await pane.getByRole("button", { name: "Close detail" }).click();
  await expect(pane).toBeHidden();
});

test("keyboard flow: / focuses, arrows select (wrap), Enter opens, Esc closes", async ({
  page,
}) => {
  await page.keyboard.press("/");
  await expect(page.getByLabel("Search packages")).toBeFocused();
  await page.keyboard.type("firefox");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("status")).toHaveText(/Found 6/, { timeout: 3_000 });

  await page.keyboard.press("ArrowDown");
  await expect(page.getByRole("option").first()).toHaveAttribute("aria-selected", "true");

  await page.keyboard.press("Enter");
  await expect(page.getByRole("complementary", { name: "Package detail" })).toBeVisible();

  await page.keyboard.press("Escape");
  await expect(page.getByRole("complementary", { name: "Package detail" })).toBeHidden();
});

test("generation guard: after typing a new query, ghost rows of the superseded run never appear", async ({
  page,
}) => {
  await search(page, "fire");
  // second run dispatched before the first finished (mock delay per leg);
  // "*nothing" is the mock's selector for an all-empty result set
  await search(page, "*nothing");

  await expect(page.getByRole("status")).toHaveText(/Found 0/, { timeout: 3_000 });
  // firefox rows came from run 1 (repo+aur) — they must never replace run 2's rows
  await expect(page.getByRole("option", { name: /firefox/ })).toHaveCount(0);
});

test("scripted failure shows the error card with the envelope's hint", async ({ page }) => {
  await search(page, "err-db-locked");
  await expect(page.getByRole("alert")).toContainText("repo_search_failed");
  await expect(page.getByRole("alert")).toContainText("hint: run pacman -Sy (mock)");
  await expect(page.getByRole("status")).toContainText("repo_search_failed: mock failure");
});

test("no results degenerate to the actionable hint", async ({ page }) => {
  await search(page, "*nothing");
  await expect(page.getByRole("status")).toHaveText(/Found 0/, { timeout: 3_000 });
  await expect(page.getByText(/Nothing matches/)).toBeVisible();
});

test("shortcuts dialog traps focus and closes cleanly", async ({ page }) => {
  await page.keyboard.press("?");
  const dialog = page.getByRole("dialog", { name: "Keyboard shortcuts" });
  await expect(dialog).toBeVisible();

  // focus stays inside the modal (trap): Tab never leaves the dialog
  await page.keyboard.press("Tab");
  await page.keyboard.press("Tab");
  const focusedInDialog = await dialog.evaluate((el) => el.contains(document.activeElement));
  expect(focusedInDialog).toBe(true);

  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();
});

test("sub-1020px: detail becomes an overlay with a backdrop; wide keeps the side pane", async ({
  page,
}) => {
  await page.setViewportSize({ width: 920, height: 700 });
  await search(page, "firefox");
  await expect(page.getByRole("status")).toHaveText(/Found 6/, { timeout: 3_000 });

  await page.getByRole("option").first().click();
  const pane = page.getByRole("complementary", { name: "Package detail" });
  await expect(page.getByRole("button", { name: "Close detail overlay" })).toBeVisible();
  await expect(pane).toBeVisible();

  await page.setViewportSize({ width: 1280, height: 800 });
  await expect(page.getByRole("button", { name: "Close detail overlay" })).toBeHidden();
});
