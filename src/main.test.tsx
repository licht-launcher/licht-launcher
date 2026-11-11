import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    minimize: vi.fn(),
    close: vi.fn(),
    setDecorations: vi.fn(),
  }),
}));

// The screens are irrelevant to startup errors, and importing them here would
// load most of the UI inside this test's timeout.
vi.mock("./routeTree.gen", async () => {
  const { createRootRoute } = await import("@tanstack/react-router");
  return { routeTree: createRootRoute() };
});
vi.mock("@/features/startup/startup-screen", () => ({
  StartupScreen: () => null,
}));

describe("frontend startup", () => {
  beforeEach(() => {
    vi.resetModules();
    window.location.hash = "#/";
  });

  it("reports a missing application root", async () => {
    await expect(import("./main")).rejects.toThrow(
      "The application root element was not found.",
    );
  });
});
