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
  within,
} from "@testing-library/react";
import type { ReactNode } from "react";
import { toast } from "sonner";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DownloadsPage } from "@/features/downloads/downloads-page";
import { downloadJob } from "@/features/downloads/fixtures/job";
import { storageSummary } from "@/features/downloads/fixtures/storage";
import {
  ACTIVE_DOWNLOAD_QUERY_KEY,
  DOWNLOAD_JOBS_QUERY_KEY,
  DownloadJobsContext,
} from "@/features/downloads/hooks/use-download-jobs";
import {
  DOWNLOAD_SPEED_HISTORY_QUERY_KEY,
  type SpeedHistory,
} from "@/features/downloads/hooks/use-download-telemetry";
import { useDownloadTelemetrySync } from "@/features/downloads/hooks/use-download-telemetry-sync";
import { snapshot } from "@/features/settings/fixtures/snapshot";
import { listDownloadJobs } from "@/lib/commands";
import type { DownloadJobSnapshot } from "@/lib/generated/DownloadJobSnapshot";
import type { DownloadTelemetry } from "@/lib/generated/DownloadTelemetry";
import { onOpenSettingsRequest } from "@/lib/shell-events";

const { invoke, listen, dispose } = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  dispose: vi.fn(),
}));
vi.mock("@tauri-apps/api/core", () => ({
  invoke,
  convertFileSrc: (path: string) => path,
}));
vi.mock("@tauri-apps/api/event", () => ({ listen }));
vi.mock("sonner", () => ({ toast: { error: vi.fn() } }));

let jobs: DownloadJobSnapshot[];
let telemetry: DownloadTelemetry;
let speedLimit: number | null;
let emit: ((event: { payload: DownloadTelemetry }) => void) | undefined;

function Jobs({ children }: { children: ReactNode }) {
  const query = useQuery({
    queryKey: DOWNLOAD_JOBS_QUERY_KEY,
    queryFn: listDownloadJobs,
  });
  useDownloadTelemetrySync(
    query.data?.some(
      (job) => job.status === "running" && job.phase === "downloading",
    ) ?? false,
  );
  return (
    <DownloadJobsContext.Provider value={query}>
      {children}
    </DownloadJobsContext.Provider>
  );
}

function renderPage() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const view = render(
    <QueryClientProvider client={client}>
      <Jobs>
        <DownloadsPage />
      </Jobs>
    </QueryClientProvider>,
  );
  return { ...view, client };
}

describe("DownloadsPage", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    invoke.mockReset();
    listen.mockReset();
    dispose.mockReset();
    vi.mocked(toast.error).mockClear();
    emit = undefined;
    jobs = [];
    speedLimit = null;
    telemetry = { receivedBytes: 0, writtenBytes: 0, paused: false };
    listen.mockImplementation(
      async (
        _name: string,
        callback: (event: { payload: DownloadTelemetry }) => void,
      ) => {
        emit = callback;
        return dispose;
      },
    );
    invoke.mockImplementation(async (command: string) => {
      if (command === "list_download_jobs") return jobs;
      if (command === "list_instances") return [];
      if (command === "download_telemetry") return telemetry;
      if (command === "storage_summary") return storageSummary;
      if (command === "get_settings")
        return {
          ...snapshot,
          settings: { ...snapshot.settings, downloadSpeedLimitMiB: speedLimit },
        };
      if (command === "pause_all_downloads") {
        telemetry = { ...telemetry, paused: true };
        const owner = jobs.find((job) => job.status === "running");
        const ordered = owner
          ? [owner, ...jobs.filter((job) => job.id !== owner.id)]
          : jobs;
        jobs = ordered.map((job) =>
          job.status === "running" || job.status === "queued"
            ? { ...job, status: "paused" }
            : job,
        );
        return;
      }
      if (command === "resume_all_downloads") {
        telemetry = { ...telemetry, paused: false };
        jobs = jobs.map((job) =>
          job.status === "paused" ? { ...job, status: "queued" } : job,
        );
        return;
      }
      throw new Error(`Unexpected command: ${command}`);
    });
  });

  it("shows a refined empty state without active and queued counters", async () => {
    renderPage();
    expect(screen.queryByText(/Loading/)).not.toBeInTheDocument();
    expect(
      await screen.findByRole("status", { name: "No downloads in progress" }),
    ).toBeVisible();
    expect(
      screen.getByText("Your next download will appear here."),
    ).toBeVisible();
    expect(screen.queryByText(/\d+ active|\d+ queued/)).not.toBeInTheDocument();
    expect(screen.queryByText("Last activity")).not.toBeInTheDocument();
    expect(
      screen.queryByRole("heading", { name: "Queue" }),
    ).not.toBeInTheDocument();
    expect(screen.queryByText(/Write speed measures/)).not.toBeInTheDocument();
    for (const label of ["Active", "Up next", "Completed"]) {
      expect(
        screen.queryByRole("region", { name: label }),
      ).not.toBeInTheDocument();
      expect(
        screen.queryByRole("heading", { name: label }),
      ).not.toBeInTheDocument();
    }
    expect(invoke).not.toHaveBeenCalledWith("list_recent_downloads");
  });

  it("shows storage below transfer speed in the independent right column", async () => {
    renderPage();
    const information = within(
      screen.getByRole("complementary", { name: "Download information" }),
    );
    const chart = information.getByRole("region", { name: "Transfer speed" });
    const storage = information.getByRole("region", { name: "Launcher Data" });
    expect(
      chart.compareDocumentPosition(storage) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(await information.findByText("20.0 GB")).toBeVisible();
  });

  it("opens the downloads section from the settings shortcut", () => {
    const requested = vi.fn();
    const unsubscribe = onOpenSettingsRequest(requested);
    renderPage();
    fireEvent.click(screen.getByRole("button", { name: "Download settings" }));
    expect(requested).toHaveBeenCalledWith("downloads");
    unsubscribe();
  });

  it("shows independent download/write measurements in one chart", async () => {
    jobs = [downloadJob()];
    const view = renderPage();
    expect(
      await screen.findByRole("region", { name: "Download for Survival" }),
    ).toBeVisible();
    act(() =>
      view.client.setQueryData<SpeedHistory>(DOWNLOAD_SPEED_HISTORY_QUERY_KEY, {
        received: [{ at: 1000, bytesPerSecond: 2_000_000 }],
        written: [{ at: 1000, bytesPerSecond: 500_000 }],
      }),
    );
    expect(
      await screen.findByRole("img", {
        name: "Transfer speed. Download: 2.0 MB/s; Write: 500.0 KB/s. Last 30 seconds.",
      }),
    ).toBeVisible();
    expect(
      within(
        screen.getByRole("region", { name: "Transfer speed" }),
      ).getAllByRole("img"),
    ).toHaveLength(1);
    act(() =>
      view.client.setQueryData<SpeedHistory>(DOWNLOAD_SPEED_HISTORY_QUERY_KEY, {
        received: [
          { at: 1000, bytesPerSecond: 2_000_000 },
          { at: 2000, bytesPerSecond: 0 },
        ],
        written: [
          { at: 1000, bytesPerSecond: 500_000 },
          { at: 2000, bytesPerSecond: 0 },
        ],
      }),
    );
    expect(
      await screen.findByRole("img", {
        name: "Transfer speed. Download: 0 B/s; Write: 0 B/s. Last 30 seconds.",
      }),
    ).toBeVisible();
    view.unmount();
    expect(dispose).toHaveBeenCalled();
  });

  it.each(["idle", "paused", "extracting"] as const)(
    "freezes the chart and retains history while %s",
    async (state) => {
      jobs = [downloadJob()];
      const { client } = renderPage();
      await screen.findByRole("region", { name: "Download for Survival" });
      const measured = {
        ...telemetry,
        receivedBytes: 2_000_000,
        writtenBytes: 500_000,
      };
      act(() =>
        client.setQueryData<SpeedHistory>(DOWNLOAD_SPEED_HISTORY_QUERY_KEY, {
          received: [{ at: 1000, bytesPerSecond: 2_000_000 }],
          written: [{ at: 1000, bytesPerSecond: 500_000 }],
        }),
      );
      const chartName =
        "Transfer speed. Download: 2.0 MB/s; Write: 500.0 KB/s. Last 30 seconds.";
      expect(await screen.findByRole("img", { name: chartName })).toBeVisible();
      act(() =>
        client.setQueryData<DownloadJobSnapshot[]>(
          DOWNLOAD_JOBS_QUERY_KEY,
          state === "idle"
            ? []
            : [
                downloadJob(
                  state === "paused"
                    ? { status: "paused" }
                    : { phase: "extracting" },
                ),
              ],
        ),
      );
      if (state === "idle")
        await screen.findByRole("status", { name: "No downloads in progress" });
      else
        await screen.findByText(
          state === "paused" ? "Paused · Minecraft" : "Extracting · Minecraft",
        );
      act(() => emit?.({ payload: measured }));
      expect(screen.getByRole("img", { name: chartName })).toBeVisible();
    },
  );

  it("shows active work, FIFO queue, failures, and bulk controls", async () => {
    jobs = [
      downloadJob({ id: "active", instanceFolder: "Active instance" }),
      downloadJob({
        id: "first",
        status: "queued",
        instanceFolder: "First waiting",
      }),
      downloadJob({
        id: "second",
        status: "queued",
        instanceFolder: "Second waiting",
      }),
      downloadJob({
        id: "failed",
        status: "failed",
        instanceFolder: "Failed instance",
        error: "Checksum mismatch",
      }),
    ];
    renderPage();
    expect(
      await screen.findByRole("region", {
        name: "Download for Active instance",
      }),
    ).toBeVisible();
    const queue = screen.getByRole("region", { name: "Up next" });
    expect(
      within(queue)
        .getAllByRole("region")
        .map((row) => row.getAttribute("aria-label")),
    ).toEqual([
      "Download for First waiting",
      "Download for Second waiting",
      "Download for Failed instance",
    ]);
    expect(screen.getByText("Checksum mismatch")).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Pause" }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("pause_all_downloads"),
    );
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Resume" })).toBeEnabled(),
    );
    expect(
      screen.queryByRole("button", { name: "Pause" }),
    ).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Resume" }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("resume_all_downloads"),
    );
    expect(await screen.findByRole("region", { name: "Queue" })).toBeVisible();
  });

  it("reports failed bulk actions", async () => {
    jobs = [downloadJob()];
    renderPage();
    await screen.findByRole("region", { name: "Download for Survival" });
    invoke.mockRejectedValueOnce(new Error("Cannot pause downloads"));
    fireEvent.click(screen.getByRole("button", { name: "Pause" }));
    await waitFor(() => expect(toast.error).toHaveBeenCalled());
  });

  it("reports unavailable live updates", async () => {
    listen.mockRejectedValueOnce(new Error("Events unavailable"));
    renderPage();
    expect(
      await screen.findByText("Live speed updates are unavailable."),
    ).toBeVisible();
  });

  it("shows plain Active and Up next headings without collapses or a Completed category", async () => {
    jobs = [
      downloadJob({ id: "active", instanceFolder: "Active instance" }),
      downloadJob({
        id: "waiting",
        instanceFolder: "Waiting instance",
        status: "queued",
      }),
    ];
    renderPage();
    await screen.findByRole("region", { name: "Download for Active instance" });
    expect(
      screen.getByRole("region", { name: "Download for Waiting instance" }),
    ).toBeVisible();
    expect(screen.queryByRole("tablist")).not.toBeInTheDocument();
    for (const label of ["Active", "Up next"]) {
      expect(
        screen.getByRole("heading", {
          name: label === "Up next" ? "Up next (1)" : label,
        }),
      ).toBeVisible();
      expect(
        screen.queryByRole("button", { name: label }),
      ).not.toBeInTheDocument();
    }
    expect(
      screen.queryByRole("region", { name: "Completed" }),
    ).not.toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalledWith("list_recent_downloads");
  });

  it.each(["running", "queued"] as const)(
    "shows only the populated category for a %s download",
    async (status) => {
      jobs = [downloadJob({ status })];
      renderPage();
      await screen.findByRole("region", { name: "Download for Survival" });
      const present = status === "running" ? "Active" : "Up next (1)";
      const absent = status === "running" ? "Up next" : "Active";
      expect(screen.getByRole("heading", { name: present })).toBeVisible();
      expect(
        screen.queryByRole("region", { name: absent }),
      ).not.toBeInTheDocument();
      expect(
        screen.queryByRole("heading", { name: absent }),
      ).not.toBeInTheDocument();
    },
  );

  it("shows the active global speed limit and updates it after settings change", async () => {
    speedLimit = 3;
    const { client } = renderPage();
    expect(await screen.findByText("Limited to 3 MiB/s")).toBeVisible();
    speedLimit = null;
    await act(async () => {
      await client.invalidateQueries({ queryKey: ["settings"] });
    });
    await waitFor(() =>
      expect(screen.queryByText(/Limited to/)).not.toBeInTheDocument(),
    );
  });

  it("uses one resume control when the global queue is paused without active items", async () => {
    telemetry = { ...telemetry, paused: true };
    renderPage();
    const resume = await screen.findByRole("button", { name: "Resume" });
    expect(resume).toBeEnabled();
    expect(
      screen.queryByRole("button", { name: "Pause" }),
    ).not.toBeInTheDocument();
    fireEvent.click(resume);
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("resume_all_downloads"),
    );
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Pause" })).toBeDisabled(),
    );
  });

  it("updates the Up next counter as items enter and leave the displayed queue", async () => {
    jobs = [
      downloadJob({ id: "active" }),
      ...["first", "second", "third"].map((id) =>
        downloadJob({ id, instanceFolder: id, status: "queued" }),
      ),
    ];
    const { client } = renderPage();
    expect(
      await screen.findByRole("heading", { name: "Up next (3)" }),
    ).toBeVisible();
    jobs = jobs.filter((job) => job.id !== "third");
    act(() => client.setQueryData(DOWNLOAD_JOBS_QUERY_KEY, jobs));
    expect(
      await screen.findByRole("heading", { name: "Up next (2)" }),
    ).toBeVisible();
    jobs = jobs.filter((job) => job.status !== "queued");
    act(() => client.setQueryData(DOWNLOAD_JOBS_QUERY_KEY, jobs));
    await waitFor(() =>
      expect(
        screen.queryByRole("region", { name: "Up next" }),
      ).not.toBeInTheDocument(),
    );
  });

  it("keeps only the last running item Active after pausing and puts the others in Up next", async () => {
    jobs = [
      downloadJob({
        id: "older",
        instanceFolder: "Older pause",
        status: "paused",
      }),
      downloadJob({ id: "owner", instanceFolder: "Current" }),
      downloadJob({ id: "next", instanceFolder: "Next", status: "queued" }),
    ];
    renderPage();
    await screen.findByRole("region", { name: "Download for Current" });
    fireEvent.click(screen.getByRole("button", { name: "Pause" }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Resume" })).toBeEnabled(),
    );
    const active = within(screen.getByRole("region", { name: "Active" }));
    expect(
      active.getByRole("region", { name: "Download for Current" }),
    ).toBeVisible();
    expect(active.getAllByRole("region")).toHaveLength(1);
    expect(
      active.getByRole("button", { name: "Resume download for Current" }),
    ).toHaveAttribute("data-variant", "ghost");
    const next = within(screen.getByRole("region", { name: "Up next" }));
    expect(
      next.getByRole("region", { name: "Download for Older pause" }),
    ).toBeVisible();
    expect(
      next.getByRole("region", { name: "Download for Next" }),
    ).toBeVisible();
    expect(
      next.getByRole("button", { name: "Reorder download for Older pause" }),
    ).toBeEnabled();
  });

  it("uses the remembered active ID while paused snapshots are arriving out of order", async () => {
    jobs = [
      downloadJob({ id: "older", instanceFolder: "Older", status: "paused" }),
      downloadJob({ id: "owner", instanceFolder: "Current" }),
    ];
    const { client } = renderPage();
    await screen.findByRole("region", { name: "Download for Current" });
    act(() => {
      client.setQueryData(ACTIVE_DOWNLOAD_QUERY_KEY, "owner");
      client.setQueryData(
        DOWNLOAD_JOBS_QUERY_KEY,
        jobs.map((job) => ({ ...job, status: "paused" })),
      );
    });
    await waitFor(() =>
      expect(
        within(screen.getByRole("region", { name: "Active" })).getByText(
          "Paused · Minecraft",
        ),
      ).toBeVisible(),
    );
    expect(
      within(screen.getByRole("region", { name: "Active" })).getByRole(
        "region",
        { name: "Download for Current" },
      ),
    ).toBeVisible();
    expect(
      within(screen.getByRole("region", { name: "Up next" })).getByRole(
        "region",
        { name: "Download for Older" },
      ),
    ).toBeVisible();
  });
});
