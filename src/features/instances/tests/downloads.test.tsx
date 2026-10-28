// Register Tauri mocks before importing telemetry, which loads command wrappers.
import "@/features/instances/tests/support/tauri-mocks";

import {
  act,
  fireEvent,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { describe, expect, it } from "vitest";

import {
  completedDownload,
  downloadJob,
} from "@/features/downloads/fixtures/job";
import {
  DOWNLOAD_SPEED_HISTORY_QUERY_KEY,
  type SpeedHistory,
} from "@/features/downloads/hooks/use-download-telemetry";
import {
  classicInstance,
  contextInstance,
  invoke,
  listeners,
  mockContextMenuCommands,
  openInstanceContextMenu,
  renderAt,
  setupInstancesPageTests,
} from "@/features/instances/tests/support/render-instances";
import type { DownloadJobSnapshot } from "@/lib/generated/DownloadJobSnapshot";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

describe("Instance downloads", () => {
  setupInstancesPageTests();

  it("continues collecting speed history in Library and restores it when returning to Downloads", async () => {
    mockContextMenuCommands((command) =>
      command === "list_download_jobs"
        ? Promise.resolve([downloadJob({ instanceFolder: "classic" })])
        : undefined,
    );
    const { router, queryClient } = await renderAt("/downloads");
    await screen.findByRole("region", { name: "Download for Classic" });
    await waitFor(() => expect(listeners.has("download-telemetry")).toBe(true));
    act(() =>
      listeners.get("download-telemetry")?.({
        payload: {
          receivedBytes: 1_000_000,
          writtenBytes: 500_000,
          paused: false,
        },
      }),
    );
    await waitFor(
      () =>
        expect(
          queryClient.getQueryData<SpeedHistory>(
            DOWNLOAD_SPEED_HISTORY_QUERY_KEY,
          )?.received.length,
        ).toBeGreaterThan(0),
      { timeout: 3000 },
    );
    const first = queryClient.getQueryData<SpeedHistory>(
      DOWNLOAD_SPEED_HISTORY_QUERY_KEY,
    )?.received[0];
    const before =
      queryClient.getQueryData<SpeedHistory>(DOWNLOAD_SPEED_HISTORY_QUERY_KEY)
        ?.received.length ?? 0;
    await act(async () => {
      fireEvent.click(
        within(
          screen.getByRole("navigation", { name: "Main navigation" }),
        ).getByRole("button", { name: "Library" }),
      );
      await router.load();
    });
    expect(
      await screen.findByRole("heading", { name: "Classic" }),
    ).toBeVisible();
    expect(
      screen.queryByRole("region", { name: "Transfer speed" }),
    ).not.toBeInTheDocument();
    act(() =>
      listeners.get("download-telemetry")?.({
        payload: {
          receivedBytes: 2_000_000,
          writtenBytes: 1_000_000,
          paused: false,
        },
      }),
    );
    await waitFor(
      () =>
        expect(
          queryClient.getQueryData<SpeedHistory>(
            DOWNLOAD_SPEED_HISTORY_QUERY_KEY,
          )?.received.length,
        ).toBeGreaterThan(before),
      { timeout: 3000 },
    );
    await act(async () => {
      fireEvent.click(
        within(
          screen.getByRole("contentinfo", { name: "Download status" }),
        ).getByRole("button", { name: /^Downloads:/ }),
      );
      await router.load();
    });
    expect(
      await screen.findByRole("region", { name: "Transfer speed" }),
    ).toBeVisible();
    expect(
      queryClient.getQueryData<SpeedHistory>(DOWNLOAD_SPEED_HISTORY_QUERY_KEY)
        ?.received[0],
    ).toEqual(first);
    expect(
      screen.getByLabelText("Download bars").querySelectorAll("rect").length,
    ).toBeGreaterThanOrEqual(2);
  });

  it("shows download progress in the footer and Downloads screen, then returns to idle", async () => {
    mockContextMenuCommands();
    await renderAt("/");
    const footer = within(
      screen.getByRole("contentinfo", { name: "Download status" }),
    );
    await footer.findByRole("button", { name: "Downloads: Idle" });
    await waitFor(() =>
      expect(listeners.has("download-job-updated")).toBe(true),
    );
    act(() =>
      listeners.get("download-job-updated")?.({ payload: downloadJob() }),
    );
    const trigger = await footer.findByRole("button", {
      name: "Downloads: 25%",
    });
    expect(trigger).not.toHaveTextContent("Downloads");
    expect(within(trigger).getByText("Downloading · Minecraft")).toBeVisible();
    expect(
      within(trigger).getByRole("progressbar", {
        name: "Current download progress",
      }),
    ).toHaveAttribute("aria-valuenow", "25");
    fireEvent.click(trigger);
    expect(await screen.findByText(/25\.0%/)).toBeInTheDocument();
    act(() =>
      listeners.get("download-job-updated")?.({ payload: completedDownload() }),
    );
    expect(
      await footer.findByRole("button", { name: "Downloads: Idle" }),
    ).toBeInTheDocument();
  });

  it.each(["paused", "interrupted", "failed"] as const)(
    "offers an explicit resume with appropriate styling for a %s download",
    async (status) => {
      let job = downloadJob({ instanceFolder: "classic", status });
      mockContextMenuCommands((command) => {
        if (command === "list_download_jobs") return Promise.resolve([job]);
        if (command === "resume_download_job") {
          job = { ...job, status: "running" };
          return Promise.resolve(job);
        }
      });
      await renderAt("/?instance=classic");
      const resume = await screen.findByRole("button", {
        name: "Resume download for Classic",
      });
      expect(resume).toHaveAttribute(
        "data-variant",
        status === "paused" ? "secondary" : "warning",
      );
      expect(invoke).not.toHaveBeenCalledWith("resume_download_job", {
        jobId: "1",
      });
      fireEvent.click(resume);
      await screen.findByRole("button", { name: "Pause download for Classic" });
      expect(invoke).toHaveBeenCalledWith("resume_download_job", {
        jobId: "1",
      });
      expect(invoke.mock.calls.some(([command]) => command === "launch")).toBe(
        false,
      );
    },
  );

  it("pauses a download on the instance page and preserves a repair after confirmed cancellation", async () => {
    let job = downloadJob({ instanceFolder: "classic", operation: "repair" });
    let cancelled = false;
    mockContextMenuCommands((command) => {
      if (command === "list_download_jobs")
        return Promise.resolve(cancelled ? [] : [job]);
      if (command === "pause_download_job") {
        job = { ...job, status: "paused" };
        return Promise.resolve(job);
      }
      if (command === "cancel_download_job") {
        cancelled = true;
        return Promise.resolve({ ...job, status: "cancelled" });
      }
    });
    await renderAt("/?instance=classic");
    fireEvent.click(
      await screen.findByRole("button", { name: "Pause download for Classic" }),
    );
    await screen.findByRole("button", { name: "Resume download for Classic" });
    fireEvent.click(
      screen.getByRole("button", { name: "Cancel download for Classic" }),
    );
    const dialog = await screen.findByRole("dialog", {
      name: "Cancel download for Classic?",
    });
    expect(dialog).toHaveTextContent(
      "Existing instances and their files will be kept.",
    );
    expect(invoke).not.toHaveBeenCalledWith("cancel_download_job", {
      jobId: "1",
    });
    fireEvent.click(
      within(dialog).getByRole("button", { name: "Cancel download" }),
    );
    await screen.findByRole("button", { name: "Play" });
    const sidebar = screen.getByRole("navigation", { name: "Instances" });
    expect(
      within(sidebar).getByRole("button", { name: /^Classic/ }),
    ).toBeVisible();
    expect(
      invoke.mock.calls.some(([command]) => command === "delete_instance"),
    ).toBe(false);
  });

  it("retries persisted cancellation from the instance page without resuming installation", async () => {
    let jobs = [
      downloadJob({
        instanceFolder: "classic",
        status: "cancelling",
        error: "Instance file is locked",
      }),
    ];
    let entries: InstanceEntry[] = [
      {
        ...classicInstance,
        installation: {
          source: { type: "vanilla", gameVersion: "1.5.2" },
          state: "installing",
        },
      },
      contextInstance,
    ];
    mockContextMenuCommands((command) => {
      if (command === "list_instances") return Promise.resolve(entries);
      if (command === "list_download_jobs") return Promise.resolve(jobs);
      if (command === "cancel_download_job") {
        const job = jobs[0];
        jobs = [];
        entries = entries.filter((entry) => entry.folder !== "classic");
        return Promise.resolve({
          ...job,
          status: "cancelled",
          error: undefined,
        });
      }
    });
    await renderAt("/?instance=classic");
    fireEvent.click(
      await screen.findByRole("button", {
        name: "Retry cancellation for Classic",
      }),
    );
    const dialog = await screen.findByRole("dialog", {
      name: "Retry cancellation for Classic?",
    });
    expect(within(dialog).getByRole("alert")).toHaveTextContent(
      "Instance file is locked",
    );
    expect(
      screen.queryByRole("button", { name: "Resume download for Classic" }),
    ).not.toBeInTheDocument();
    fireEvent.click(
      within(dialog).getByRole("button", { name: "Retry cancellation" }),
    );
    await screen.findByRole("heading", { name: "Next" });
    expect(invoke).toHaveBeenCalledWith("cancel_download_job", { jobId: "1" });
    expect(
      invoke.mock.calls.some(
        ([command]) =>
          command === "resume_download_job" ||
          command === "prepare_instance_download" ||
          command === "launch",
      ),
    ).toBe(false);
  });

  it.each(["instance page", "downloads screen"])(
    "removes a new instance only after confirming cancellation from the %s",
    async (location) => {
      let entries: InstanceEntry[] = [
        {
          ...classicInstance,
          installation: {
            source: { type: "vanilla", gameVersion: "1.5.2" },
            state: "installing",
          },
        },
        contextInstance,
      ];
      let jobs = [
        downloadJob({ instanceFolder: "classic", versionId: "1.5.2" }),
      ];
      mockContextMenuCommands((command) => {
        if (command === "list_instances") return Promise.resolve(entries);
        if (command === "list_download_jobs") return Promise.resolve(jobs);
        if (command === "cancel_download_job") {
          const job = jobs[0];
          jobs = [];
          entries = entries.filter((entry) => entry.folder !== "classic");
          return Promise.resolve({ ...job, status: "cancelled" });
        }
      });
      await renderAt("/?instance=classic");
      if (location === "downloads screen") {
        fireEvent.click(
          await screen.findByRole("button", { name: "Downloads: 25%" }),
        );
        const row = await screen.findByRole("region", {
          name: "Download for Classic",
        });
        fireEvent.click(
          within(row).getByRole("button", {
            name: "Cancel download for Classic",
          }),
        );
      } else {
        fireEvent.click(
          await screen.findByRole("button", {
            name: "Cancel download for Classic",
          }),
        );
      }
      const dialog = await screen.findByRole("dialog", {
        name: "Cancel download for Classic?",
      });
      expect(dialog).toHaveTextContent(
        "removed from your library, along with its files",
      );
      expect(invoke).not.toHaveBeenCalledWith("cancel_download_job", {
        jobId: "1",
      });
      const before = screen.getByLabelText(
        location === "downloads screen" ? "Download for Classic" : "Instances",
      );
      expect(within(before).getByText("Classic")).toBeInTheDocument();
      fireEvent.click(
        within(dialog).getByRole("button", { name: "Cancel download" }),
      );
      await waitFor(() =>
        expect(
          screen.queryByRole("dialog", {
            name: "Cancel download for Classic?",
          }),
        ).not.toBeInTheDocument(),
      );
      if (location === "downloads screen") {
        fireEvent.click(screen.getByRole("button", { name: "Library" }));
      }
      const sidebar = await screen.findByRole("navigation", {
        name: "Instances",
      });
      await waitFor(() =>
        expect(within(sidebar).queryByText("Classic")).not.toBeInTheDocument(),
      );
      expect(within(sidebar).getByText("Next")).toBeInTheDocument();
      expect(invoke).toHaveBeenCalledWith("cancel_download_job", {
        jobId: "1",
      });
    },
  );

  it("prepares a missing version, reports progress and then launches it", async () => {
    let finishInstall: (job: DownloadJobSnapshot) => void = () => {};
    mockContextMenuCommands((command) => {
      if (command === "is_version_ready_cmd") return Promise.resolve(false);
      if (command === "prepare_instance_download")
        return new Promise<DownloadJobSnapshot>((resolve) => {
          finishInstall = resolve;
        });
      if (command === "launch") return Promise.resolve({ code: 0 });
    });
    await renderAt("/");
    fireEvent.click(await screen.findByRole("button", { name: "Play" }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("prepare_instance_download", {
        folder: "classic",
        operation: "install",
      }),
    );
    expect(
      await screen.findByRole("progressbar", { name: "Install progress" }),
    ).toBeVisible();
    expect(invoke).not.toHaveBeenCalledWith("launch", expect.anything());
    const menu = await openInstanceContextMenu("Classic");
    expect(
      within(menu).getByRole("menuitem", { name: "Installing…" }),
    ).toHaveAttribute("aria-disabled", "true");
    expect(
      within(menu).getByRole("menuitem", { name: "Duplicate" }),
    ).toHaveAttribute("aria-disabled", "true");
    fireEvent.keyDown(menu, { key: "Escape" });
    await waitFor(() =>
      expect(screen.queryByRole("menu")).not.toBeInTheDocument(),
    );
    act(() =>
      listeners.get("install-progress")?.({
        payload: {
          versionId: "1.5.2",
          finished: 0,
          failed: 0,
          total: 0,
          bytesDone: 0,
          bytesTotal: 0,
        },
      }),
    );
    expect(await screen.findByText("Preparing…")).toBeVisible();
    act(() =>
      listeners.get("install-progress")?.({
        payload: {
          versionId: "1.5.2",
          finished: 1,
          failed: 0,
          total: 4,
          bytesDone: 5_000_000,
          bytesTotal: 10_000_000,
        },
      }),
    );
    expect(await screen.findByText(/50\.0%\s+5\.0MB\/10\.0MB/)).toBeVisible();
    await act(async () =>
      finishInstall(
        completedDownload({ instanceFolder: "classic", versionId: "1.5.2" }),
      ),
    );
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("launch", {
        folder: "classic",
        username: "Steve",
      }),
    );
  });

  it("does not launch when preparation is cancelled", async () => {
    mockContextMenuCommands((command) => {
      if (command === "is_version_ready_cmd") return Promise.resolve(false);
      if (command === "prepare_instance_download")
        return Promise.resolve(
          downloadJob({ status: "cancelled", instanceFolder: "classic" }),
        );
    });
    await renderAt("/");
    fireEvent.click(await screen.findByRole("button", { name: "Play" }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("prepare_instance_download", {
        folder: "classic",
        operation: "install",
      }),
    );
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Play" })).toBeEnabled(),
    );
    expect(invoke).not.toHaveBeenCalledWith("launch", expect.anything());
  });

  it("resolves an unprepared modpack before allowing an explicit launch", async () => {
    let ready = false;
    let finish: (() => void) | undefined;
    const pending: InstanceEntry = {
      ...classicInstance,
      name: "Pending pack",
      versionId: null,
      installation: {
        state: "preparing",
        source: {
          type: "modpack",
          source: "modrinth",
          projectId: "pack",
          versionId: "pack-version",
          iconUrl: null,
        },
      },
    };
    mockContextMenuCommands((command) => {
      if (command === "list_instances")
        return Promise.resolve([
          ready
            ? {
                ...pending,
                versionId: "fabric-loader-0.16.10-1.21.1",
                installation: { ...pending.installation, state: "ready" },
              }
            : pending,
        ]);
      if (command === "is_version_ready_cmd") return Promise.resolve(ready);
      if (command === "prepare_instance_download")
        return new Promise<DownloadJobSnapshot>((resolve) => {
          finish = () => {
            ready = true;
            resolve(
              completedDownload({
                instanceFolder: "classic",
                versionId: "fabric-loader-0.16.10-1.21.1",
              }),
            );
          };
        });
      if (command === "launch") return Promise.resolve({ code: 0 });
    });
    await renderAt("/");
    fireEvent.click(await screen.findByRole("button", { name: "Install" }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("prepare_instance_download", {
        folder: "classic",
        operation: "install",
      }),
    );
    expect(screen.getByRole("button", { name: "Install" })).toBeDisabled();
    expect(invoke).not.toHaveBeenCalledWith("launch", expect.anything());
    if (!finish) throw new Error("Preparation was not started");
    await act(async () => finish?.());
    const play = await screen.findByRole("button", { name: "Play" });
    expect(invoke).not.toHaveBeenCalledWith("launch", expect.anything());
    fireEvent.click(play);
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("launch", {
        folder: "classic",
        username: "Steve",
      }),
    );
  });

  it("accepts two installation requests and keeps progress isolated by version", async () => {
    const finishInstall: Record<string, (job: DownloadJobSnapshot) => void> =
      {};
    mockContextMenuCommands((command, args) => {
      if (command === "is_version_ready_cmd") return Promise.resolve(false);
      if (
        command === "prepare_instance_download" &&
        args !== null &&
        typeof args === "object" &&
        "folder" in args &&
        typeof args.folder === "string"
      ) {
        const folder = args.folder;
        return new Promise<DownloadJobSnapshot>((resolve) => {
          finishInstall[folder] = resolve;
        });
      }
      if (command === "launch") return Promise.resolve({ code: 0 });
    });
    await renderAt("/");
    await screen.findByRole("heading", { name: "Classic" });
    const sidebar = within(
      screen.getByRole("navigation", { name: "Instances" }),
    );
    for (const [name, folder] of [
      ["Classic", "classic"],
      ["Next", "next"],
    ]) {
      fireEvent.click(sidebar.getByRole("button", { name }));
      fireEvent.click(await screen.findByRole("button", { name: "Play" }));
      await waitFor(() =>
        expect(invoke).toHaveBeenCalledWith("prepare_instance_download", {
          folder,
          operation: "install",
        }),
      );
    }
    act(() =>
      listeners.get("install-progress")?.({
        payload: {
          versionId: "1.20.1",
          finished: 1,
          failed: 0,
          total: 2,
          bytesDone: 4_000_000,
          bytesTotal: 8_000_000,
        },
      }),
    );
    expect(await screen.findByText(/50\.0%\s+4\.0MB\/8\.0MB/)).toBeVisible();
    fireEvent.click(sidebar.getByRole("button", { name: "Classic" }));
    expect(screen.queryByText(/4\.0MB\/8\.0MB/)).not.toBeInTheDocument();
    act(() =>
      listeners.get("install-progress")?.({
        payload: {
          versionId: "1.5.2",
          finished: 1,
          failed: 0,
          total: 4,
          bytesDone: 5_000_000,
          bytesTotal: 10_000_000,
        },
      }),
    );
    expect(await screen.findByText(/50\.0%\s+5\.0MB\/10\.0MB/)).toBeVisible();
    await act(async () => {
      finishInstall.classic?.(
        completedDownload({ instanceFolder: "classic", versionId: "1.5.2" }),
      );
      finishInstall.next?.(completedDownload({ instanceFolder: "next" }));
    });
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("launch", {
        folder: "classic",
        username: "Steve",
      });
      expect(invoke).toHaveBeenCalledWith("launch", {
        folder: "next",
        username: "Steve",
      });
    });
  });
});
