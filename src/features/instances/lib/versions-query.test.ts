import { describe, expect, it } from "vitest";

import {
  VERSIONS_QUERY_KEY,
  VERSIONS_STALE_TIME,
  versionsQueryOptions,
} from "@/features/instances/lib/versions-query";

describe("versionsQueryOptions", () => {
  it("keeps the manifest list until explicitly invalidated", () => {
    expect(VERSIONS_QUERY_KEY).toEqual(["versions"]);
    expect(VERSIONS_STALE_TIME).toBe(Number.POSITIVE_INFINITY);
    expect(versionsQueryOptions.staleTime).toBe(Number.POSITIVE_INFINITY);
    expect(versionsQueryOptions.retry).toBe(false);
  });
});
