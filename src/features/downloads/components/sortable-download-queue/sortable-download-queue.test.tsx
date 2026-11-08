import {
  QueryClient,
  QueryClientProvider,
  useQuery,
} from "@tanstack/react-query";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import type { ReactNode } from "react";
import { toast } from "sonner";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { SortableDownloadQueue } from "@/features/downloads/components/sortable-download-queue";
import { downloadJob } from "@/features/downloads/fixtures/job";
import { DOWNLOAD_JOBS_QUERY_KEY } from "@/features/downloads/hooks/use-download-jobs";
import { listDownloadJobs } from "@/lib/commands";
import type { DownloadJobSnapshot } from "@/lib/generated/DownloadJobSnapshot";

const { invoke, drag } = vi.hoisted(() => ({
  invoke: vi.fn(),
  drag: { canceled: false, destination: 1 },
}));
vi.mock("@tauri-apps/api/core", () => ({
  invoke,
  convertFileSrc: (path: string) => path,
}));
vi.mock("sonner", () => ({ toast: { error: vi.fn() } }));
type DragFixture = {
  canceled: boolean;
  operation: {
    source: { id: string; initialIndex: number; index: number };
    target: { id: string };
  };
};
vi.mock("@dnd-kit/react", () => ({
  DragDropProvider: ({
    children,
    onDragStart,
    onDragEnd,
  }: {
    children: ReactNode;
    onDragStart: () => void;
    onDragEnd: (event: DragFixture) => void;
  }) => (
    <div>
      <button type="button" onClick={onDragStart}>
        Start drag
      </button>
      <button
        type="button"
        onClick={() =>
          onDragEnd({
            canceled: drag.canceled,
            operation: {
              source: { id: "a", initialIndex: 0, index: drag.destination },
              target: { id: "b" },
            },
          })
        }
      >
        Drop
      </button>
      {children}
    </div>
  ),
}));
vi.mock("@dnd-kit/react/sortable", () => ({
  useSortable: () => ({
    ref: () => {},
    handleRef: () => {},
    isDragSource: false,
  }),
  isSortableOperation: () => true,
}));

let stored: DownloadJobSnapshot[];
let rejectReorder: boolean;

function Queue() {
  const query = useQuery({
    queryKey: DOWNLOAD_JOBS_QUERY_KEY,
    queryFn: listDownloadJobs,
  });
  return (
    <SortableDownloadQueue
      jobs={(query.data ?? []).filter((job) => job.id !== "running")}
      instances={[]}
    />
  );
}

function renderQueue() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false, staleTime: Infinity } },
  });
  client.setQueryData(DOWNLOAD_JOBS_QUERY_KEY, stored);
  render(
    <QueryClientProvider client={client}>
      <Queue />
    </QueryClientProvider>,
  );
  return client;
}

describe("SortableDownloadQueue", () => {
  beforeEach(() => {
    invoke.mockReset();
    vi.mocked(toast.error).mockClear();
    drag.canceled = false;
    drag.destination = 1;
    rejectReorder = false;
    stored = [
      downloadJob({ id: "running", instanceFolder: "Running" }),
      downloadJob({ id: "a", instanceFolder: "First", status: "queued" }),
      downloadJob({ id: "b", instanceFolder: "Second", status: "queued" }),
    ];
    invoke.mockImplementation(
      async (command: string, args?: Record<string, unknown>) => {
        if (command === "list_download_jobs") return stored;
        if (command !== "reorder_download_jobs")
          throw new Error(`Unexpected command ${command}`);
        if (rejectReorder)
          throw new Error("Download queue changed; refresh and try again");
        const ids = args?.jobIds;
        if (!Array.isArray(ids)) throw new Error("Missing IDs");
        const waiting = ids.flatMap((id: unknown) =>
          typeof id === "string" ? stored.filter((job) => job.id === id) : [],
        );
        stored = [
          ...stored.filter(
            (job) => !waiting.some((pending) => pending.id === job.id),
          ),
          ...waiting,
        ];
        return stored;
      },
    );
  });

  it("persists a completed drag and preserves the running job", async () => {
    const client = renderQueue();
    const handle = screen.getByRole("button", {
      name: "Reorder download for First",
    });
    handle.focus();
    expect(handle).toHaveFocus();
    fireEvent.click(screen.getByRole("button", { name: "Start drag" }));
    fireEvent.click(screen.getByRole("button", { name: "Drop" }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("reorder_download_jobs", {
        jobIds: ["b", "a"],
      }),
    );
    await waitFor(() =>
      expect(
        screen
          .getAllByRole("region")
          .map((row) => row.getAttribute("aria-label")),
      ).toEqual(["Download for Second", "Download for First"]),
    );
    expect(
      client.getQueryData<DownloadJobSnapshot[]>(DOWNLOAD_JOBS_QUERY_KEY)?.[0]
        ?.id,
    ).toBe("running");
  });

  it("restores the actual queue when the backend rejects a reorder", async () => {
    rejectReorder = true;
    const client = renderQueue();
    fireEvent.click(screen.getByRole("button", { name: "Start drag" }));
    fireEvent.click(screen.getByRole("button", { name: "Drop" }));
    await waitFor(() => expect(toast.error).toHaveBeenCalled());
    await waitFor(() =>
      expect(
        client
          .getQueryData<DownloadJobSnapshot[]>(DOWNLOAD_JOBS_QUERY_KEY)
          ?.map((job) => job.id),
      ).toEqual(["running", "a", "b"]),
    );
  });

  it.each(["canceled", "unchanged"])("does not persist a %s drag", (kind) => {
    drag.canceled = kind === "canceled";
    drag.destination = kind === "unchanged" ? 0 : 1;
    renderQueue();
    fireEvent.click(screen.getByRole("button", { name: "Start drag" }));
    fireEvent.click(screen.getByRole("button", { name: "Drop" }));
    expect(invoke).not.toHaveBeenCalled();
  });

  it("rejects a drag if the queued jobs change during the interaction", async () => {
    const client = renderQueue();
    fireEvent.click(screen.getByRole("button", { name: "Start drag" }));
    act(() =>
      client.setQueryData(DOWNLOAD_JOBS_QUERY_KEY, [
        ...stored,
        downloadJob({ id: "c", instanceFolder: "Third", status: "queued" }),
      ]),
    );
    await screen.findByRole("region", { name: "Download for Third" });
    fireEvent.click(screen.getByRole("button", { name: "Drop" }));
    expect(invoke).not.toHaveBeenCalled();
    expect(toast.error).toHaveBeenCalledWith(
      "Download queue changed; try moving the download again.",
    );
  });

  it("keeps a single waiting item draggable without persisting a no-op drop", () => {
    stored = stored.filter((job) => job.id !== "b");
    drag.destination = 0;
    renderQueue();
    const handle = screen.getByRole("button", {
      name: "Reorder download for First",
    });
    expect(handle).toBeEnabled();
    handle.focus();
    expect(handle).toHaveFocus();
    fireEvent.click(screen.getByRole("button", { name: "Start drag" }));
    fireEvent.click(screen.getByRole("button", { name: "Drop" }));
    expect(invoke).not.toHaveBeenCalled();
  });

  it("reorders paused items without resuming them or moving the retained active row", async () => {
    stored = stored.map((job) => ({ ...job, status: "paused" }));
    const client = renderQueue();
    fireEvent.click(screen.getByRole("button", { name: "Start drag" }));
    fireEvent.click(screen.getByRole("button", { name: "Drop" }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("reorder_download_jobs", {
        jobIds: ["b", "a"],
      }),
    );
    await waitFor(() =>
      expect(
        client
          .getQueryData<DownloadJobSnapshot[]>(DOWNLOAD_JOBS_QUERY_KEY)
          ?.map((job) => job.id),
      ).toEqual(["running", "b", "a"]),
    );
    expect(
      client
        .getQueryData<DownloadJobSnapshot[]>(DOWNLOAD_JOBS_QUERY_KEY)
        ?.every((job) => job.status === "paused"),
    ).toBe(true);
  });
});
