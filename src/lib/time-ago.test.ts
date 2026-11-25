import { describe, expect, it } from "vitest";

import { formatTimeAgo } from "./time-ago";

describe("formatTimeAgo", () => {
  const now = Date.parse("2026-11-10T12:00:00Z");

  it("names the largest whole unit that has passed", () => {
    expect(formatTimeAgo("2026-11-10T11:59:30Z", now)).toBe("just now");
    expect(formatTimeAgo("2026-11-10T11:15:00Z", now)).toBe("45 minutes ago");
    expect(formatTimeAgo("2026-11-10T09:00:00Z", now)).toBe("3 hours ago");
    expect(formatTimeAgo("2026-11-09T10:00:00Z", now)).toBe("yesterday");
    expect(formatTimeAgo("2026-11-01T10:43:19.109719+00:00", now)).toBe(
      "last week",
    );
    expect(formatTimeAgo("2026-08-01T00:00:00Z", now)).toBe("3 months ago");
    expect(formatTimeAgo("2025-04-02T07:44:02.187Z", now)).toBe("last year");
  });

  it("counts whole units for a date slightly ahead of the clock", () => {
    expect(formatTimeAgo("2026-11-10T13:30:00Z", now)).toBe("in 1 hour");
  });

  it("returns null when there is no readable date", () => {
    expect(formatTimeAgo(null, now)).toBeNull();
    expect(formatTimeAgo("soon", now)).toBeNull();
  });
});
