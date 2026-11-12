import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useSearchDraft } from "./use-search-draft";

describe("useSearchDraft", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("sends the trimmed text once typing pauses", () => {
    const onSearch = vi.fn();
    const { result } = renderHook(() => useSearchDraft("", onSearch));

    act(() => result.current.change("cob"));
    act(() => result.current.change("cobble "));
    expect(result.current.draft).toBe("cobble ");
    expect(onSearch).not.toHaveBeenCalled();

    act(() => vi.advanceTimersByTime(300));
    expect(onSearch).toHaveBeenCalledExactlyOnceWith("cobble");
  });

  it("does not resend a query that only differs by spaces", () => {
    const onSearch = vi.fn();
    const { result } = renderHook(() => useSearchDraft("cobble", onSearch));

    act(() => result.current.change("cobble  "));
    act(() => vi.advanceTimersByTime(300));
    expect(onSearch).not.toHaveBeenCalled();
  });

  it("follows an outside query change and drops the pending send", () => {
    const onSearch = vi.fn();
    const { result, rerender } = renderHook(
      ({ query }) => useSearchDraft(query, onSearch),
      { initialProps: { query: "cobble" } },
    );

    act(() => result.current.change("cobblemon"));
    rerender({ query: "" });
    act(() => vi.advanceTimersByTime(300));

    expect(result.current.draft).toBe("");
    expect(onSearch).not.toHaveBeenCalled();
  });
});
