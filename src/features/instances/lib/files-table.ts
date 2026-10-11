import type { InstanceFsEntry } from "@/lib/generated/InstanceFsEntry";

export type FilesSortKey = "name" | "size" | "created";
export type FilesSortDir = "asc" | "desc";

export function formatFileSize(size: number | null): string {
  if (size === null) {
    return "—";
  }
  if (size < 1024) {
    return `${size} B`;
  }
  if (size < 1024 * 1024) {
    return `${(size / 1024).toFixed(1)} KB`;
  }
  return `${(size / (1024 * 1024)).toFixed(1)} MB`;
}

export function formatFileCreated(created: number | null): string {
  if (created === null) {
    return "—";
  }
  return new Date(created).toLocaleString(undefined, {
    year: "numeric",
    month: "numeric",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  });
}

export function sortFileEntries(
  entries: InstanceFsEntry[],
  key: FilesSortKey,
  dir: FilesSortDir,
): InstanceFsEntry[] {
  const sign = dir === "asc" ? 1 : -1;
  return [...entries].sort((left, right) => {
    if (left.isDir !== right.isDir) {
      return left.isDir ? -1 : 1;
    }
    let compare = 0;
    switch (key) {
      case "name":
        compare = left.name.localeCompare(right.name, undefined, {
          sensitivity: "base",
        });
        break;
      case "size":
        compare = (left.size ?? -1) - (right.size ?? -1);
        break;
      case "created":
        compare = (left.created ?? 0) - (right.created ?? 0);
        break;
    }
    return compare * sign;
  });
}

export function toggleFilesSort(
  currentKey: FilesSortKey,
  currentDir: FilesSortDir,
  nextKey: FilesSortKey,
): { key: FilesSortKey; dir: FilesSortDir } {
  if (currentKey === nextKey) {
    return { key: currentKey, dir: currentDir === "asc" ? "desc" : "asc" };
  }
  return { key: nextKey, dir: "asc" };
}
