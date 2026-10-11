import { describe, expect, it } from "vitest";

import {
  formatFileCreated,
  sortFileEntries,
  toggleFilesSort,
} from "@/features/instances/lib/files-table";
import type { InstanceFsEntry } from "@/lib/generated/InstanceFsEntry";

const entries: InstanceFsEntry[] = [
  { name: "zeta.txt", isDir: false, size: 20, created: 3000 },
  { name: "alpha", isDir: true, size: null, created: 1000 },
  { name: "beta.txt", isDir: false, size: 10, created: 2000 },
];

describe("formatFileCreated", () => {
  it("omits seconds from the formatted timestamp", () => {
    const formatted = formatFileCreated(Date.UTC(2024, 0, 15, 12, 34, 56));
    expect(formatted).not.toContain(":56");
  });

  it("returns an em dash when created is missing", () => {
    expect(formatFileCreated(null)).toBe("—");
  });
});

describe("sortFileEntries", () => {
  it("keeps directories first and sorts by name", () => {
    const sorted = sortFileEntries(entries, "name", "asc");
    expect(sorted.map((entry) => entry.name)).toEqual([
      "alpha",
      "beta.txt",
      "zeta.txt",
    ]);
  });

  it("sorts files by size while keeping directories first", () => {
    const sorted = sortFileEntries(entries, "size", "asc");
    expect(sorted.map((entry) => entry.name)).toEqual([
      "alpha",
      "beta.txt",
      "zeta.txt",
    ]);
  });

  it("sorts by created descending", () => {
    const sorted = sortFileEntries(entries, "created", "desc");
    expect(sorted.map((entry) => entry.name)).toEqual([
      "alpha",
      "zeta.txt",
      "beta.txt",
    ]);
  });
});

describe("toggleFilesSort", () => {
  it("flips direction for the same column and resets for a new one", () => {
    expect(toggleFilesSort("name", "asc", "name")).toEqual({
      key: "name",
      dir: "desc",
    });
    expect(toggleFilesSort("name", "desc", "size")).toEqual({
      key: "size",
      dir: "asc",
    });
  });
});
