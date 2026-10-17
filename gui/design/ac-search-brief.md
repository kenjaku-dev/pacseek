# AC design brief — pakmap-gui search experience

Wireframe-level spec for the Phase AC search screen. Approval checkpoint happens at
**structure level only** — color/branding/theme is Phase AE (tokens shared with the
landing `design-system/`). This doc follows the `design-process` briefing pattern.

## Briefing

- **Current state:** the TUI delivers the bar — staged streaming (repo → AUR → flatpak),
  generation-guarded, repo leg paints first (~150–440 ms), AUR warm pool at launch,
  detail view with badges `[OOD]/[ORPHAN]/[UNMAINTAINED]`, orphans filter, readonly
  mode. The GUI spike (AB) has the three IPC round trips but no real UI.
- **Goals (measurable):**
  - G1 staged paint works like the TUI: repo leg visible < 500 ms debug / < ~440 ms
    release, no join-all-before-paint.
  - G2 every parity-matrix filter reachable with ≤ 2 interactions (source, aur_by,
    regex, installed_only, orphans_only, limit).
  - G3 detail pane shows: description, version, repo, installed/update badge, AUR
    popularity votes, flatpak app id, package base.
  - G4 keyboard-complete: search, navigate, filter toggle, detail open — zero mouse
    required; mouse-complete too.
  - G5 generation guard visible: typing fast never mixes results from two searches.
- **Target audience:** Arch/Artix users who live in pacman/AUR daily — they expect
  information density, keyboard flow, and zero hand-holding. They read package lists
  the way the TUI draws them: name+version+repo+installed-state at a glance.
- **Constraints:** WebKitGTK system webview (no pointer:dark magic beyond CSS), tokens
  shared with landing (AE), Tailwind 4, no runtime deps beyond React (state via
  reducer + context), `config.tui.debounce_ms` semantics reused, budgets NFR-1.
- **Competition:** pamac (cute but heavy/slow), octopi (utilitarian, flat lists),
  bauh, gnome-software (store-y, low density). **pakmap deliberately differs:** TUI
  density in a GUI, staged-streaming honesty (you SEE the legs arrive), keyboard-first.
- **Success criteria:** exit criteria in the plan + user sign-off at this checkpoint.

## Information architecture (AC window)

Single-window app, one screen in AC; tab bar nodes for Updates/News land in AD.

```text
┌────────────────────────────────────────────────────────────────┐
│  pakmap            [ search input with debounce ▁▁▁▁▁ ]  ⌕    │  header (64px)
├────────────────────────────────────────────────────────────────┤
│  [All] [Repo] [AUR] [Flatpak]  regex ⊙  installed ⊙  orphans ⊙ │  filter bar (48px)
├───────────────────────────────────────┬────────────────────────┤
│  ▸ staged groups:                      │  DETAIL PANE            │
│    Repo (24)     ← paints first        │  ─ name + badges row    │
│    AUR (57)      ← legs arrive in      │  ─ version · repo       │
│      old generations dropped           │  ─ popularity votes     │
│    Flatpak (4)                         │  ─ description          │
│  ─────────┬─ list rows, 12px rhythm ─  │  ─ files/deps (info)    │
│  ■ firefox  139.0  repo   inst         │  ─ [row actions]        │
│  ■ firefox-bin 139.0 aur  —            │    (install/remove      │
│  ■ Firefox  flatpak —                  │     arrive in AD)       │
│                                        │                         │
│  status: Found 85 (repo 24 aur 57 flatpak 4) · 320ms             │  footer strip
└────────────────────────────────────────────────────────────────┘
```

## Structural rules (wireframe level)

1. **Header search field** = primary focus, always visible, autoselect-on-focus,
   `debounce_ms` from config; Enter forces immediate search (no debounce wait).
2. **Filter bar**: segmented source control (All/Repo/AUR/Flatpak — filters the
   *displayed groups*, stream keeps running), then toggle chips: regex, installed
   only, orphans only. Limit is config-driven, not a visible control (v0.9 if asked).
3. **Staged groups**: three group blocks, ordered Repo → AUR → Flatpak; each has a
   count badge that ticks as its leg streams in. A group "arrives" as a unit.
   Superseded generations must never mix (same invariant as TUI `staged_packages`).
4. **Master-detail**: list left (≥ 55% width, min window 900×620 — pane collapses to
   overlay detail on narrow windows), detail pane right; rows 1-line dense
   (name, version, repo tag, installed/ood badges), 12px vertical rhythm.
5. **Detail pane**: title + badges row, meta line, popularity for AUR, description,
   collapsible files/deps via `package_info`.
6. **Footer strip**: `Found N (repo X aur Y flatpak Z)` exact TUI wording + elapsed
   ms — parity with TUI status wording is the contract.
7. **Keyboard**: `/` or focus → search; ↑/↓ rows (wrap at group edge), PgUp/PgDn,
   Enter → open detail (install action arrives in AD), Esc → clear search,
   `Tab` → filters, `?` → shortcuts overlay.
8. **State matrix (each designed, never defaulted):** empty/first-run (hints + hotkey
   legend), searching (streaming skeleton with live leg counters), partial (some
   groups still streaming), done, no-results (query + "try fewer filters" hint),
   error (envelope card: code + message + hint), offline (AUR leg error, repo/flatpak
   still fine — degraded, not dead).

## Reducer contract hint (for implementation)

One typed reducer consumes `search://stage` events; state =
`{ generation, groups: { repo, aur, flatpak }[], status }`; first event of a new
generation REPLACES groups (atomic per leg), later events append; vitest fixtures
mirror the 4 TUI staged tests incl. ghost-generation discard.

## Deliberately out of AC scope (backlog, not Frankenstein material)

Tabs for Updates/News, row actions (install/remove buttons + progress), theme system,
lightbox animations, onboarding tour. New asks go to the backlog (v0.9).
