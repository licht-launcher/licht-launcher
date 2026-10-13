import { afterEach, describe, expect, it, vi } from "vitest";

import { sidebarLayoutStorage } from "@/features/instances/lib/sidebar-layout";

describe("sidebarLayoutStorage", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    localStorage.clear();
  });

  it("reads and writes through localStorage", () => {
    sidebarLayoutStorage.setItem("k", "v");
    expect(sidebarLayoutStorage.getItem("k")).toBe("v");
  });

  it("returns null when getItem throws", () => {
    vi.stubGlobal("localStorage", {
      getItem: () => {
        throw new Error("blocked");
      },
      setItem: () => undefined,
    });
    expect(sidebarLayoutStorage.getItem("k")).toBeNull();
  });

  it("swallows setItem errors", () => {
    vi.stubGlobal("localStorage", {
      getItem: () => null,
      setItem: () => {
        throw new Error("quota");
      },
    });
    expect(() => {
      sidebarLayoutStorage.setItem("k", "v");
    }).not.toThrow();
  });
});
