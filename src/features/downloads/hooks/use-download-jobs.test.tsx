import {
  QueryClient,
  QueryClientProvider,
  useQuery,
} from "@tanstack/react-query";
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
  ACTIVE_DOWNLOAD_QUERY_KEY,
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
    expect(listen).toHaveBeenCalledTimes(3);
    expect(listen).toHaveBeenCalledWith(
      "download-telemetry",
      expect.any(Function),
    );
  });

  it("remembers the running owner while paused snapshots arrive", async () => {
    const older = downloadJob({ id: "old", status: "paused" });
    const owner = downloadJob({ id: "owner" });
    invoke.mockResolvedValue([older, owner]);
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={client}>
        <DownloadJobsProvider>{children}</DownloadJobsProvider>
      </QueryClientProvider>
    );
    const { result } = renderHook(() => useDownloadJobs(), { wrapper });
    await waitFor(() => expect(result.current.data).toHaveLength(2));
    expect(client.getQueryData(ACTIVE_DOWNLOAD_QUERY_KEY)).toBe("owner");
    act(() =>
      listeners.get("download-job-updated")?.({
        payload: { ...owner, status: "paused" },
      }),
    );
    expect(client.getQueryData(ACTIVE_DOWNLOAD_QUERY_KEY)).toBe("owner");
  });

  it("restores the paused owner from persisted queue priority", async () => {
    invoke.mockResolvedValue([
      downloadJob({ id: "owner", status: "paused" }),
      downloadJob({ id: "older", status: "paused" }),
    ]);
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    client.setQueryData(ACTIVE_DOWNLOAD_QUERY_KEY, "older");
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={client}>
        <DownloadJobsProvider>{children}</DownloadJobsProvider>
      </QueryClientProvider>
    );
    const { result } = renderHook(() => useDownloadJobs(), { wrapper });
    await waitFor(() => expect(result.current.data).toHaveLength(2));
    expect(client.getQueryData(ACTIVE_DOWNLOAD_QUERY_KEY)).toBe("owner");
  });

  it("refreshes persisted FIFO order when a paused installation rejoins the queue", async () => {
    const first = downloadJob({ status: "paused" });
    const second = downloadJob({
      id: "2",
      instanceFolder: "Second",
      status: "queued",
    });
    let stored = [first, second];
    invoke.mockImplementation(async () => stored);
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
    await waitFor(() => expect(result.current.data).toEqual([first, second]));
    const resumed = { ...first, status: "queued" as const };
    stored = [second, resumed];
    act(() => listeners.get("download-job-updated")?.({ payload: resumed }));
    await waitFor(() => expect(result.current.data).toEqual([second, resumed]));
    unmount();
  });

  it.each([
    ["install", "completed"],
    ["install", "failed"],
    ["install", "cancelled"],
    ["repair", "completed"],
    ["repair", "failed"],
    ["repair", "cancelled"],
  ] as const)(
    "refreshes only the affected content once when %s becomes %s",
    async (operation, status) => {
      invoke.mockResolvedValue([]);
      const client = new QueryClient({
        defaultOptions: { queries: { retry: false } },
      });
      const key = ["instance-content", "Survival"];
      client.setQueryData(key, ["cached mod"]);
      client.setQueryData(["instance-content", "Other"], ["other mod"]);
      let finish: ((entries: string[]) => void) | undefined;
      const loadContent = vi.fn(
        () =>
          new Promise<string[]>((resolve) => {
            finish = resolve;
          }),
      );
      const wrapper = ({ children }: { children: ReactNode }) => (
        <QueryClientProvider client={client}>
          <DownloadJobsProvider>{children}</DownloadJobsProvider>
        </QueryClientProvider>
      );
      const { result, unmount } = renderHook(
        () => ({
          jobs: useDownloadJobs(),
          content: useQuery({
            queryKey: key,
            queryFn: loadContent,
            staleTime: Number.POSITIVE_INFINITY,
          }),
        }),
        { wrapper },
      );
      await waitFor(() => expect(result.current.jobs.data).toEqual([]));
      const emit = (snapshot: DownloadJobSnapshot) =>
        act(() =>
          listeners.get("download-job-updated")?.({ payload: snapshot }),
        );
      emit(downloadJob({ operation }));
      expect(loadContent).not.toHaveBeenCalled();
      emit(downloadJob({ operation, status }));
      await waitFor(() => expect(loadContent).toHaveBeenCalledTimes(1));
      expect(result.current.content.data).toEqual(["cached mod"]);
      expect(result.current.content.isFetching).toBe(true);
      await act(async () => {
        if (!finish) throw new Error("missing content refresh");
        finish(["updated mod"]);
      });
      await waitFor(() =>
        expect(result.current.content.data).toEqual(["updated mod"]),
      );
      emit(downloadJob({ operation, status, updatedAt: 1_700_000_002_000 }));
      expect(loadContent).toHaveBeenCalledTimes(1);
      expect(
        client.getQueryState(["instance-content", "Other"])?.isInvalidated,
      ).toBe(false);
      if (status === "failed") {
        emit(downloadJob({ operation }));
        emit(downloadJob({ operation, status }));
        expect(loadContent).toHaveBeenCalledTimes(2);
        await act(async () => {
          if (!finish) throw new Error("missing retry content refresh");
          finish(["partially repaired mod"]);
        });
        await waitFor(() =>
          expect(result.current.content.data).toEqual([
            "partially repaired mod",
          ]),
        );
      }
      unmount();
    },
  );

  it("marks inactive content stale even when completion is the first event", async () => {
    invoke.mockResolvedValue([]);
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const key = ["instance-content", "Survival"];
    client.setQueryData(key, ["cached mod"]);
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={client}>
        <DownloadJobsProvider>{children}</DownloadJobsProvider>
      </QueryClientProvider>
    );
    const { result, unmount } = renderHook(() => useDownloadJobs(), {
      wrapper,
    });
    await waitFor(() => expect(result.current.data).toEqual([]));
    act(() =>
      listeners.get("download-job-updated")?.({ payload: completedDownload() }),
    );
    expect(client.getQueryState(key)?.isInvalidated).toBe(true);
    expect(client.getQueryData(key)).toEqual(["cached mod"]);
    unmount();
  });

  it("does not invalidate content for a completed runtime-only job", async () => {
    invoke.mockResolvedValue([]);
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const key = ["instance-content", "Survival"];
    client.setQueryData(key, ["cached mod"]);
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={client}>
        <DownloadJobsProvider>{children}</DownloadJobsProvider>
      </QueryClientProvider>
    );
    const { result, unmount } = renderHook(() => useDownloadJobs(), {
      wrapper,
    });
    await waitFor(() => expect(result.current.data).toEqual([]));
    act(() =>
      listeners.get("download-job-updated")?.({
        payload: completedDownload({ instanceFolder: null, javaMajor: 21 }),
      }),
    );
    expect(client.getQueryState(key)?.isInvalidated).toBe(false);
    unmount();
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
    vi.mocked(listen).mockImplementation((name, handler, options) => {
      if (name === "download-jobs-invalidated")
        return Promise.reject(new Error("event bridge unavailable"));
      return subscribe(name, handler, options);
    });
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
    expect(listeners.has("download-job-updated")).toBe(false);
    expect(listeners.has("download-jobs-invalidated")).toBe(false);
    expect(listeners.has("download-telemetry")).toBe(true);
    invoke.mockResolvedValue([downloadJob()]);
    await act(async () => {
      await result.current.refetch();
    });
    await waitFor(() =>
      expect(result.current.data?.[0]?.status).toBe("running"),
    );
    unmount();
    expect(listeners.size).toBe(0);
    vi.mocked(listen).mockImplementation(subscribe);
  });
});
