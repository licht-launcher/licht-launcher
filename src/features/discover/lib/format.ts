import type { ModpackVersionItem } from "@/lib/generated/ModpackVersionItem";

const downloadsFormat = new Intl.NumberFormat("en", {
  notation: "compact",
  maximumFractionDigits: 1,
});

const dateFormat = new Intl.DateTimeFormat("en", { dateStyle: "medium" });

const relativeFormat = new Intl.RelativeTimeFormat("en", { numeric: "auto" });

const RELATIVE_UNITS: readonly [Intl.RelativeTimeFormatUnit, number][] = [
  ["year", 365 * 24 * 60 * 60_000],
  ["month", 30 * 24 * 60 * 60_000],
  ["week", 7 * 24 * 60 * 60_000],
  ["day", 24 * 60 * 60_000],
  ["hour", 60 * 60_000],
  ["minute", 60_000],
];

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

/**
 * How long ago an RFC 3339 timestamp was, e.g. `3 days ago`; `null` when it
 * cannot be read. `now` is a parameter so callers and tests share one clock.
 */
export function formatTimeAgo(
  timestamp: string | null,
  now: number,
): string | null {
  if (timestamp === null) {
    return null;
  }
  const time = new Date(timestamp).getTime();
  if (Number.isNaN(time)) {
    return null;
  }
  const elapsed = now - time;
  for (const [unit, size] of RELATIVE_UNITS) {
    if (Math.abs(elapsed) >= size) {
      return relativeFormat.format(-Math.floor(elapsed / size), unit);
    }
  }
  return "just now";
}
