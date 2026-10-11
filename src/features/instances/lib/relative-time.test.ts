import { describe, expect, it } from "vitest";

import { formatRelativeOpenedAt } from "@/features/instances/lib/relative-time";

const NOW = Date.parse("2026-10-11T15:00:00Z");

describe("formatRelativeOpenedAt", () => {
  it("returns null for missing or invalid stamps", () => {
    expect(formatRelativeOpenedAt(null, NOW)).toBeNull();
    expect(formatRelativeOpenedAt(undefined, NOW)).toBeNull();
    expect(formatRelativeOpenedAt("", NOW)).toBeNull();
    expect(formatRelativeOpenedAt("not-a-date", NOW)).toBeNull();
  });

  it("formats short relative spans", () => {
    expect(formatRelativeOpenedAt("2026-10-11T14:59:30Z", NOW)).toBe(
      "Just now",
    );
    expect(formatRelativeOpenedAt("2026-10-11T14:55:00Z", NOW)).toBe(
      "5 minutes ago",
    );
    expect(formatRelativeOpenedAt("2026-10-11T14:00:00Z", NOW)).toBe(
      "1 hour ago",
    );
    expect(formatRelativeOpenedAt("2026-10-11T05:00:00Z", NOW)).toBe(
      "10 hours ago",
    );
    expect(formatRelativeOpenedAt("2026-10-09T15:00:00Z", NOW)).toBe(
      "2 days ago",
    );
    expect(formatRelativeOpenedAt("2026-09-27T15:00:00Z", NOW)).toBe(
      "2 weeks ago",
    );
  });

  it("falls back to a UTC date after about a month", () => {
    expect(formatRelativeOpenedAt("2026-08-01T12:00:00Z", NOW)).toBe(
      "2026-08-01",
    );
  });
});
