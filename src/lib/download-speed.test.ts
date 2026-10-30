import { describe, expect, it } from "vitest";
import {
  downloadLimitKbs,
  formatDownloadLimit,
  parseDownloadLimit,
} from "@/lib/download-speed";

describe("download speed units", () => {
  it.each([
    ["", null],
    ["1000", 1_000_000],
    ["0.001", 1],
    [".5", 500],
    ["1048.576", 1_048_576],
  ])("parses %s kB/s as an exact byte rate", (input, bytes) => {
    expect(parseDownloadLimit(input)).toBe(bytes);
  });

  it.each(["0", "-1", "abc", "Infinity", "1e3", "0.0001", "9007199254740.992"])(
    "rejects invalid rate %s",
    (value) => {
      expect(parseDownloadLimit(value)).toBe("invalid");
    },
  );

  it.each([
    1,
    1000,
    1_048_576,
    3_145_728,
    4_503_599_626_321_920,
    Number.MAX_SAFE_INTEGER,
  ])("preserves %s bytes/s through the input", (bytes) => {
    expect(parseDownloadLimit(downloadLimitKbs(bytes))).toBe(bytes);
  });

  it("formats the supporting text in MiB/s without reporting zero for a positive limit", () => {
    expect(formatDownloadLimit(1_000_000)).toBe("0.95 MiB/s");
    expect(formatDownloadLimit(3 * 1024 * 1024)).toBe("3 MiB/s");
    expect(formatDownloadLimit(1)).toBe("< 0.01 MiB/s");
    expect(downloadLimitKbs(null)).toBe("");
  });
});
