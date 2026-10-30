import { describe, expect, it } from "vitest";

import { formatMemoryPreset, memoryPresetOptions } from "@/lib/memory-presets";

describe("memoryPresetOptions", () => {
  it("provides sparse marks including the minimum on an 8 GB machine", () => {
    expect(
      memoryPresetOptions({
        minimumMb: 512,
        maximumMb: 8192,
        recommendedMb: 4096,
        totalMb: 8192,
      }),
    ).toEqual([512, 2048, 4096, 6144, 8192]);
  });

  it("stops at the machine ceiling on 4 GB", () => {
    expect(
      memoryPresetOptions({
        minimumMb: 512,
        maximumMb: 4096,
        recommendedMb: 2048,
        totalMb: 4096,
      }),
    ).toEqual([512, 1024, 2048, 3072, 4096]);
  });

  it("keeps the marks readable on 16 GB machines", () => {
    expect(
      memoryPresetOptions({
        minimumMb: 512,
        maximumMb: 16384,
        recommendedMb: 4096,
        totalMb: 16384,
      }),
    ).toEqual([512, 4096, 8192, 12288, 16384]);
  });

  it("retains the exact RAM endpoint above 16 GB", () => {
    expect(
      memoryPresetOptions({
        minimumMb: 512,
        maximumMb: 32701,
        recommendedMb: 4096,
        totalMb: 32701,
      }),
    ).toEqual([512, 8192, 16384, 24576, 32701]);
  });
});

describe("formatMemoryPreset", () => {
  it.each([
    { mb: 512, label: "512MB" },
    { mb: 1024, label: "1GB" },
    { mb: 1536, label: "1.5GB" },
    { mb: 2048, label: "2GB" },
    { mb: 32701, label: "31.9GB" },
    { mb: 32768, label: "32GB" },
  ])("formats $mb MB as $label", ({ mb, label }) => {
    expect(formatMemoryPreset(mb)).toBe(label);
  });
});
