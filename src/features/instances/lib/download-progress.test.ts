import { describe, expect, it } from "vitest";

import {
  isPreparingProgress,
  progressText,
} from "@/features/instances/lib/download-progress";

describe("download progress helpers", () => {
  it("treats missing or empty totals as preparing", () => {
    expect(isPreparingProgress(null)).toBe(true);
    expect(isPreparingProgress(undefined)).toBe(true);
    expect(
      isPreparingProgress({
        finished: 0,
        failed: 0,
        total: 0,
        bytesDone: 0,
        bytesTotal: 0,
      }),
    ).toBe(true);
    expect(
      isPreparingProgress({
        finished: 1,
        failed: 0,
        total: 4,
        bytesDone: 1,
        bytesTotal: 10,
      }),
    ).toBe(false);
  });

  it("formats a mid-download progress line", () => {
    expect(
      progressText({
        finished: 1,
        failed: 0,
        total: 4,
        bytesDone: 5_000_000,
        bytesTotal: 10_000_000,
      }),
    ).toMatch(/50\.0%\s+5\.0MB\/10\.0MB/);
  });
});
