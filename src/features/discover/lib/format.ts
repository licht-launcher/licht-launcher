import { LOADER_LABELS } from "@/features/discover/lib/filter-options";
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

/** Fits the install select; longer names end in `...` and keep the suffix. */
const VERSION_LABEL_LIMIT = 48;

function versionDetails(version: ModpackVersionItem): string {
  const loaders = version.loaders.map(
    (loader) => LOADER_LABELS.get(loader) ?? loader,
  );
  return [version.gameVersions.join(", "), loaders.join(", ")]
    .filter((part) => part !== "")
    .join(" · ");
}

/** Version picker label, e.g. `7.0.0 (1.21.10 · Fabric)`. */
export function formatVersionLabel(version: ModpackVersionItem): string {
  const details = versionDetails(version);
  return details === "" ? version.name : `${version.name} (${details})`;
}

function clip(value: string, limit: number): string {
  let head = value.slice(0, limit).trimEnd();
  // A cut that lands on "RELEASE -" would read as "RELEASE -...".
  while (head.endsWith("-") || head.endsWith(",") || head.endsWith("·")) {
    head = head.slice(0, -1).trimEnd();
  }
  return head;
}

function withEllipsis(value: string, limit: number): string {
  if (value.length <= limit) {
    return value;
  }
  return `${clip(value, Math.max(0, limit - 3))}...`;
}

/**
 * Label shown in the install select. A long version name is cut with `...`
 * and the game version and loader stay visible when they fit.
 */
export function displayVersionLabel(version: ModpackVersionItem): string {
  const details = versionDetails(version);
  const suffix = details === "" ? "" : ` (${details})`;
  const full = `${version.name}${suffix}`;
  if (full.length <= VERSION_LABEL_LIMIT) {
    return full;
  }
  const nameLimit = VERSION_LABEL_LIMIT - suffix.length - 3;
  if (nameLimit < 1) {
    return withEllipsis(full, VERSION_LABEL_LIMIT);
  }
  const head = clip(version.name, nameLimit);
  if (head === "") {
    return withEllipsis(full, VERSION_LABEL_LIMIT);
  }
  return `${head}...${suffix}`;
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
      // Truncate toward zero, so a date ahead of the clock counts the same way.
      return relativeFormat.format(-Math.trunc(elapsed / size), unit);
    }
  }
  return "just now";
}
