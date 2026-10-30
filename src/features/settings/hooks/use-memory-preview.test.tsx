import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { snapshot } from "@/features/settings/fixtures/snapshot";
import {
  MEMORY_PREVIEW_QUERY_KEY,
  setMemoryPreview,
  useMemoryPreview,
} from "@/features/settings/hooks/use-memory-preview";

function renderPreview(client = new QueryClient()) {
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );
  return { ...renderHook(useMemoryPreview, { wrapper }), client };
}

describe("useMemoryPreview", () => {
  afterEach(() => vi.unstubAllEnvs());

  it("starts with real RAM and keeps simulated edits across section remounts", async () => {
    const first = renderPreview();
    expect(first.result.current.totalMb).toBeNull();
    act(() => setMemoryPreview(first.client, 64 * 1024));
    await waitFor(() => expect(first.result.current.totalMb).toBe(65536));
    act(() => first.result.current.setAllocationMb(50001, 512));
    await waitFor(() => expect(first.result.current.allocationMb).toBe(50001));
    first.unmount();
    const reopened = renderPreview(first.client);
    expect(reopened.result.current.totalMb).toBe(65536);
    expect(reopened.result.current.allocationMb).toBe(50001);
    act(() => setMemoryPreview(first.client, null));
    await waitFor(() => expect(reopened.result.current.totalMb).toBeNull());
    expect(reopened.result.current.allocationMb).toBeNull();
  });

  it("resets allocation when switching RAM sizes without changing saved settings", async () => {
    const client = new QueryClient();
    client.setQueryData(["settings"], snapshot);
    setMemoryPreview(client, 128 * 1024);
    const preview = renderPreview(client);
    act(() => preview.result.current.setAllocationMb(100001, 512));
    await waitFor(() =>
      expect(preview.result.current.allocationMb).toBe(100001),
    );
    act(() => setMemoryPreview(client, 4 * 1024));
    await waitFor(() => expect(preview.result.current.totalMb).toBe(4096));
    expect(preview.result.current.allocationMb).toBeNull();
    expect(client.getQueryData(["settings"])).toEqual(snapshot);
  });

  it("rejects unsupported sizes and allocations outside the preview limits", () => {
    const preview = renderPreview();
    expect(() => setMemoryPreview(preview.client, 999)).toThrow(
      "Unsupported RAM preview size",
    );
    act(() => setMemoryPreview(preview.client, 4096));
    expect(() => preview.result.current.setAllocationMb(511, 512)).toThrow(
      "outside the simulated RAM range",
    );
    expect(() => preview.result.current.setAllocationMb(4097, 512)).toThrow(
      "outside the simulated RAM range",
    );
  });

  it("always uses real memory in production even when a preview exists in cache", () => {
    const client = new QueryClient();
    setMemoryPreview(client, 65536);
    vi.stubEnv("DEV", false);
    setMemoryPreview(client, 131072);
    const preview = renderPreview(client);
    expect(preview.result.current.totalMb).toBeNull();
    expect(preview.result.current.allocationMb).toBeNull();
    expect(client.getQueryData(MEMORY_PREVIEW_QUERY_KEY)).toEqual({
      totalMb: 65536,
      allocationMb: null,
    });
  });
});
