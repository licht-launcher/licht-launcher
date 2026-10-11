const MINUTE_MS = 60_000;
const HOUR_MS = 60 * MINUTE_MS;
const DAY_MS = 24 * HOUR_MS;
const WEEK_MS = 7 * DAY_MS;
const MONTH_MS = 30 * DAY_MS;

/**
 * Relative English label for when an instance was last opened.
 * Returns null when the stamp is missing or unparsable.
 */
export function formatRelativeOpenedAt(
  iso: string | null | undefined,
  nowMs: number = Date.now(),
): string | null {
  if (iso === null || iso === undefined || iso.trim() === "") {
    return null;
  }
  const openedMs = Date.parse(iso);
  if (Number.isNaN(openedMs)) {
    return null;
  }
  const elapsed = Math.max(0, nowMs - openedMs);

  if (elapsed < MINUTE_MS) {
    return "Just now";
  }
  if (elapsed < HOUR_MS) {
    const minutes = Math.floor(elapsed / MINUTE_MS);
    return minutes === 1 ? "1 minute ago" : `${minutes} minutes ago`;
  }
  if (elapsed < DAY_MS) {
    const hours = Math.floor(elapsed / HOUR_MS);
    return hours === 1 ? "1 hour ago" : `${hours} hours ago`;
  }
  if (elapsed < WEEK_MS) {
    const days = Math.floor(elapsed / DAY_MS);
    return days === 1 ? "1 day ago" : `${days} days ago`;
  }
  if (elapsed < MONTH_MS) {
    const weeks = Math.floor(elapsed / WEEK_MS);
    return weeks === 1 ? "1 week ago" : `${weeks} weeks ago`;
  }

  const date = new Date(openedMs);
  const year = date.getUTCFullYear();
  const month = String(date.getUTCMonth() + 1).padStart(2, "0");
  const day = String(date.getUTCDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}
