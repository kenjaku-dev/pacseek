// @vitest-environment jsdom
// useTheme — device-level theme preference: boot resolution (stored pin beats
// OS), dataset application, and the live system-follow listener.
import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  applyResolved,
  readStoredMode,
  resolveTheme,
  useTheme,
  type ThemeMode,
} from "./useTheme";

type MediaStub = { matches: boolean };
let light: MediaStub = { matches: false };

function installMatchMedia(): void {
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    writable: true,
    value: (query: string) => {
      if (!query.includes("prefers-color-scheme")) {
        return {
          matches: false,
          addEventListener: () => {},
          removeEventListener: () => {},
        };
      }
      return {
        matches: light.matches,
        addEventListener: (_t: string, cb: () => void) => {
          listeners.add(cb);
        },
        removeEventListener: (_t: string, cb: () => void) => {
          listeners.delete(cb);
        },
      };
    },
  });
}

const listeners = new Set<() => void>();

function flipSystem(toLight: boolean): void {
  light = { matches: toLight };
  for (const cb of listeners) cb();
}

function harness(initialMode: ThemeMode | null) {
  if (initialMode === null) localStorage.removeItem("pakmap-theme");
  else localStorage.setItem("pakmap-theme", initialMode);
  return renderHook(() => useTheme());
}

describe("useTheme", () => {
  beforeEach(() => {
    listeners.clear();
    light = { matches: false };
    installMatchMedia();
    localStorage.clear();
    delete document.documentElement.dataset.theme;
  });

  afterEach(() => {
    localStorage.clear();
  });

  it("defaults to system and resolves dark on a dark OS", () => {
    const h = harness(null);
    expect(h.result.current.mode).toBe("system");
    expect(h.result.current.resolved).toBe("dark");
    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(readStoredMode()).toBe("system");
  });

  it("follows a light OS when system-pinned", () => {
    light = { matches: true };
    const h = harness(null);
    expect(h.result.current.resolved).toBe("light");
    expect(document.documentElement.dataset.theme).toBe("light");
  });

  it("a stored pin wins over the OS", () => {
    const h = harness("dark");
    expect(h.result.current.mode).toBe("dark");
    expect(h.result.current.resolved).toBe("dark");
    expect(document.documentElement.dataset.theme).toBe("dark");
  });

  it("setMode persists and applies", () => {
    const h = harness(null);
    act(() => h.result.current.setMode("light"));
    expect(h.result.current.mode).toBe("light");
    expect(localStorage.getItem("pakmap-theme")).toBe("light");
    expect(document.documentElement.dataset.theme).toBe("light");
  });

  it("re-pinning system rejoins the OS and live-updates on its change", async () => {
    const h = harness("light");
    act(() => h.result.current.setMode("system"));
    expect(h.result.current.resolved).toBe("dark"); // OS starts dark
    await act(async () => flipSystem(true));
    expect(h.result.current.resolved).toBe("light"); // OS change follows through
    expect(document.documentElement.dataset.theme).toBe("light");
  });

  it("pinned dark is unaffected by OS flips", () => {
    const h = harness("dark");
    flipSystem(true);
    expect(h.result.current.resolved).toBe("dark");
    expect(document.documentElement.dataset.theme).toBe("dark");
  });

  it("resolveTheme is pure for explicit modes", () => {
    expect(resolveTheme("light")).toBe("light");
    expect(resolveTheme("dark")).toBe("dark");
  });

  it("applyResolved writes the dataset", () => {
    applyResolved("light");
    expect(document.documentElement.dataset.theme).toBe("light");
  });
});
