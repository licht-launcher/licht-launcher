import { describe, expect, it } from "vitest";
import { appUpdateState } from "@/features/updater/fixtures/state";
import { parseAppUpdateState } from "@/lib/app-update-state";

describe("application update state boundary", () => {
  it("accepts the generated state shape", () => {
    const state = appUpdateState({
      phase: "downloading",
      version: "1.2.3",
      downloadedBytes: 25,
      totalBytes: 100,
    });
    expect(parseAppUpdateState(state)).toEqual(state);
  });
  it.each([
    null,
    {},
    { ...appUpdateState(), phase: "unknown" },
    { ...appUpdateState(), automaticUpdates: "yes" },
    { ...appUpdateState(), downloadedBytes: -1 },
    { ...appUpdateState(), totalBytes: Number.MAX_SAFE_INTEGER + 1 },
  ])("rejects an invalid IPC payload", (payload) => {
    expect(() => parseAppUpdateState(payload)).toThrow(
      "Invalid application update state",
    );
  });
});
