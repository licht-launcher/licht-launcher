import { describe, expect, it } from "vitest";

import { isInstallProgressActive } from "@/hooks/use-install-activity";

describe("isInstallProgressActive", () => {
  it("ignores the zero-total preparing pulse", () => {
    expect(
      isInstallProgressActive({
        versionId: "fabric-loader-0.16.0-1.20.1",
        finished: 0,
        failed: 0,
        total: 0,
        bytesDone: 0,
        bytesTotal: 0,
      }),
    ).toBe(false);
  });

  it("is active while files remain", () => {
    expect(
      isInstallProgressActive({
        versionId: "1.20.1",
        finished: 1,
        failed: 0,
        total: 4,
        bytesDone: 10,
        bytesTotal: 40,
      }),
    ).toBe(true);
  });

  it("is inactive when finished", () => {
    expect(
      isInstallProgressActive({
        versionId: "1.20.1",
        finished: 4,
        failed: 0,
        total: 4,
        bytesDone: 40,
        bytesTotal: 40,
      }),
    ).toBe(false);
  });
});
