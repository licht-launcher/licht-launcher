import { beforeEach, describe, expect, it, vi } from "vitest";

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
