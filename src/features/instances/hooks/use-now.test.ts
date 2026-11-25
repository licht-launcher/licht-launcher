import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useNow } from "./use-now";

describe("useNow", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(Date.parse("2026-11-25T12:00:00Z"));
  });

  afterEach(() => vi.useRealTimers());

  it("moves forward on each interval", () => {
    const { result } = renderHook(() => useNow(60_000));
    expect(result.current).toBe(Date.parse("2026-11-25T12:00:00Z"));

    act(() => vi.advanceTimersByTime(60_000));

    expect(result.current).toBe(Date.parse("2026-11-25T12:01:00Z"));
  });
});
