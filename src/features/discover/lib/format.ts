import type { ModpackVersionItem } from "@/lib/generated/ModpackVersionItem";

const downloadsFormat = new Intl.NumberFormat("en", {
  notation: "compact",
  maximumFractionDigits: 1,
});

const dateFormat = new Intl.DateTimeFormat("en", { dateStyle: "medium" });

/** Download count as shown in store listings, e.g. `19.2M`. */
export function formatDownloads(downloads: number): string {
  return downloadsFormat.format(downloads);
}

/** Calendar date of an RFC 3339 timestamp; `null` when it cannot be read. */
export function formatUpdatedAt(timestamp: string | null): string | null {
  if (timestamp === null) {
    return null;
  }
  const date = new Date(timestamp);
  return Number.isNaN(date.getTime()) ? null : dateFormat.format(date);
}

/** Version picker label, e.g. `7.0.0 (1.21.10 · fabric)`. */
export function formatVersionLabel(version: ModpackVersionItem): string {
  const details = [version.gameVersions.join(", "), version.loaders.join(", ")]
    .filter((part) => part !== "")
    .join(" · ");
  return details === "" ? version.name : `${version.name} (${details})`;
}
