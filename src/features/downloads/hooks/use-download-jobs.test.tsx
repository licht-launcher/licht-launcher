import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import { act, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DownloadJobsProvider } from "@/features/downloads/components/download-jobs-provider";

import {
  completedDownload,
  downloadJob,
} from "@/features/downloads/fixtures/job";
import {
  isDownloadFinished,
  replaceDownloadSnapshot,
  useDownloadJobs,
} from "@/features/downloads/hooks/use-download-jobs";
import type { DownloadJobSnapshot } from "@/lib/generated/DownloadJobSnapshot";

const { invoke, listeners } = vi.hoisted(() => ({
  invoke: vi.fn(),
  listeners: new Map<string, (event: { payload: unknown }) => void>(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(
    async (name: string, handler: (event: { payload: unknown }) => void) => {
      listeners.set(name, handler);
      return () => {
        listeners.delete(name);
      };
    },
  ),
}));

describe("download task activity", () => {
  beforeEach(() => {
    listeners.clear();
    invoke.mockReset();
    vi.mocked(listen).mockClear();
  });

  it("keeps preparing and byte-complete tasks active until the job completes", () => {
    expect(
      isDownloadFinished(
        downloadJob({
          phase: "preparing",
          progress: {
            finished: 0,
            failed: 0,
            total: 0,
            bytesDone: 0,
            bytesTotal: 0,
          },
        }),
      ),
    ).toBe(false);
    expect(
      isDownloadFinished(
        completedDownload({ status: "running", phase: "extracting" }),
      ),
    ).toBe(false);
    expect(isDownloadFinished(completedDownload())).toBe(true);
    expect(isDownloadFinished(downloadJob({ status: "cancelled" }))).toBe(true);
  });

  it("tracks two tasks for the same version separately and removes only the terminal task", () => {
    const first = downloadJob();
    const second = downloadJob({ id: "2", instanceFolder: "Second" });
    const both = replaceDownloadSnapshot([first], second);
    expect(both).toEqual([first, second]);
    expect(replaceDownloadSnapshot(both, completedDownload())).toEqual([
      second,
    ]);
    expect(
      replaceDownloadSnapshot(both, downloadJob({ status: "paused" }))[0],
    ).toEqual(downloadJob({ status: "paused" }));
  });

  it("preserves live updates arriving while the initial persisted snapshot is loading", async () => {
    let finish: ((jobs: DownloadJobSnapshot[]) => void) | undefined;
    invoke.mockImplementation(
      () =>
        new Promise<DownloadJobSnapshot[]>((resolve) => {
          finish = resolve;
        }),
    );
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={client}>
        <DownloadJobsProvider>{children}</DownloadJobsProvider>
      </QueryClientProvider>
    );
    const { result, unmount } = renderHook(() => useDownloadJobs(), {
      wrapper,
    });
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("list_download_jobs"),
    );
    act(() =>
      listeners.get("download-job-updated")?.({ payload: downloadJob() }),
    );
    await act(async () => {
      if (!finish) throw new Error("missing snapshot request");
      finish([]);
    });
    await waitFor(() => expect(result.current.data).toEqual([downloadJob()]));
    unmount();
    expect(listeners.size).toBe(0);
  });

  it("subscribes once when multiple consumers use the same downloads query", async () => {
    invoke.mockResolvedValue([]);
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={client}>
        <DownloadJobsProvider>{children}</DownloadJobsProvider>
      </QueryClientProvider>
    );
    const { result } = renderHook(
      () => ({ first: useDownloadJobs(), second: useDownloadJobs() }),
      { wrapper },
    );
    await waitFor(() => expect(result.current.first.data).toEqual([]));
    expect(result.current.second.data).toEqual([]);
    expect(listen).toHaveBeenCalledTimes(2);
  });

  it("uses a newer database snapshot instead of an event received before the fetch", async () => {
    invoke.mockResolvedValue([downloadJob()]);
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={client}>
        <DownloadJobsProvider>{children}</DownloadJobsProvider>
      </QueryClientProvider>
    );
    const { result } = renderHook(() => useDownloadJobs(), { wrapper });
    await waitFor(() => expect(result.current.data).toEqual([downloadJob()]));
    act(() =>
      listeners.get("download-job-updated")?.({
        payload: downloadJob({ status: "paused" }),
      }),
    );
    await waitFor(() =>
      expect(result.current.data?.[0]?.status).toBe("paused"),
    );
    const stored = downloadJob({
      progress: {
        finished: 2,
        failed: 0,
        total: 4,
        bytesDone: 200_000,
        bytesTotal: 400_000,
      },
    });
    invoke.mockResolvedValue([stored]);
    await act(async () => {
      await result.current.refetch();
    });
    await waitFor(() => expect(result.current.data).toEqual([stored]));
  });

  it("cleans up a partial subscription and keeps database recovery available", async () => {
    const subscribe = vi.mocked(listen).getMockImplementation();
    if (!subscribe) throw new Error("missing event mock");
    vi.mocked(listen)
      .mockImplementationOnce(subscribe)
      .mockRejectedValueOnce(new Error("event bridge unavailable"));
    invoke.mockResolvedValue([downloadJob({ status: "paused" })]);
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={client}>
        <DownloadJobsProvider>{children}</DownloadJobsProvider>
      </QueryClientProvider>
    );
    const { result, unmount } = renderHook(() => useDownloadJobs(), {
      wrapper,
    });
    await waitFor(() =>
      expect(result.current.data?.[0]?.status).toBe("paused"),
    );
    expect(listeners.size).toBe(0);
    invoke.mockResolvedValue([downloadJob()]);
    await act(async () => {
      await result.current.refetch();
    });
    await waitFor(() =>
      expect(result.current.data?.[0]?.status).toBe("running"),
    );
    unmount();
    expect(listeners.size).toBe(0);
  });
});
