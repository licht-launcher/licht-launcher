import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import type { ComponentProps } from "react";
import { toast } from "sonner";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DownloadJobsProvider } from "@/features/downloads/components/download-jobs-provider";
import { DownloadsPopover } from "@/features/downloads/components/downloads-popover";
import { downloadJob } from "@/features/downloads/fixtures/job";
import type { DownloadJobSnapshot } from "@/lib/generated/DownloadJobSnapshot";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({
  invoke,
  convertFileSrc: (path: string) => `asset://localhost/${path}`,
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));
vi.mock("sonner", () => ({ toast: { error: vi.fn() } }));

const instance: InstanceEntry = {
  folder: "Survival",
  name: "My Survival",
  versionId: "1.20.1",
  minMemoryMb: 512,
  maxMemoryMb: 2048,
  jvmArguments: [],
  fullscreen: false,
  width: null,
  height: null,
  overrideWindow: false,
  overrideMemory: false,
  overrideJava: false,
  overrideJvmArguments: false,
  javaPath: null,
  lastOpenedAt: null,
  iconPath: "/icons/survival.png",
};

function backend(initial: DownloadJobSnapshot[]) {
  let jobs = initial;
  invoke.mockImplementation(
    async (command: string, args?: Record<string, unknown>) => {
      if (command === "list_download_jobs") return jobs;
      if (command === "list_instances") return [instance];
      const current = jobs.find((job) => job.id === args?.jobId);
      if (!current) throw new Error("job missing");
      const status =
        command === "pause_download_job"
          ? "paused"
          : command === "resume_download_job"
            ? "running"
            : command === "cancel_download_job"
              ? "cancelled"
              : null;
      if (status === null) throw new Error(`unexpected command ${command}`);
      const result: DownloadJobSnapshot = { ...current, status };
      jobs = status === "cancelled" ? [] : [result];
      return result;
    },
  );
}

function renderDownloads(props: ComponentProps<typeof DownloadsPopover> = {}) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <DownloadJobsProvider>
        <DownloadsPopover {...props} />
      </DownloadJobsProvider>
    </QueryClientProvider>,
  );
}

describe("DownloadsPopover", () => {
  beforeEach(() => {
    invoke.mockReset();
    vi.mocked(toast.error).mockReset();
  });

  it("opens the downloads screen from the footer shortcut", async () => {
    backend([]);
    const onOpen = vi.fn();
    renderDownloads({ showLabel: true, onOpen });
    fireEvent.click(
      await screen.findByRole("button", { name: "Downloads: Idle" }),
    );
    expect(onOpen).toHaveBeenCalledOnce();
  });

  it("shows the running installation and queued count in the footer summary", async () => {
    backend([
      downloadJob({ id: "1", status: "queued", instanceFolder: "Waiting" }),
      downloadJob({ id: "2", instanceFolder: instance.folder }),
      downloadJob({ id: "3", status: "queued", instanceFolder: "Later" }),
    ]);
    renderDownloads({ showLabel: true, side: "top" });
    const trigger = await screen.findByRole("button", { name: "Downloads: 3" });
    expect(trigger).not.toHaveTextContent("Downloads");
    expect(within(trigger).getByText("Downloading · Minecraft")).toBeVisible();
    expect(within(trigger).getByText("2 queued")).toBeVisible();
    expect(within(trigger).getByText("25%")).toBeVisible();
    expect(
      within(trigger).getByRole("progressbar", {
        name: "Current download progress",
      }),
    ).toHaveAttribute("aria-valuenow", "25");
    fireEvent.click(trigger);
    expect(
      await screen.findByRole("region", { name: "Download for My Survival" }),
    ).toBeVisible();
    expect(
      screen.getByRole("region", { name: "Download for Waiting" }),
    ).toBeVisible();
    expect(
      screen.getByRole("region", { name: "Download for Later" }),
    ).toBeVisible();
  });

  it("keeps paused progress visible and does not invent progress for queued work", async () => {
    backend([downloadJob({ status: "paused" })]);
    const paused = renderDownloads({ showLabel: true });
    const trigger = await screen.findByRole("button", {
      name: "Downloads: Paused",
    });
    expect(within(trigger).getByText("Paused · Minecraft")).toBeVisible();
    expect(within(trigger).getByText("25%")).toBeVisible();
    paused.unmount();
    backend([downloadJob({ status: "queued" })]);
    renderDownloads({ showLabel: true });
    const waiting = await screen.findByRole("button", {
      name: "Downloads: Queued",
    });
    expect(within(waiting).getByText("Queued · Minecraft")).toBeVisible();
    expect(within(waiting).queryByText("25%")).not.toBeInTheDocument();
    expect(
      within(waiting).getByRole("progressbar", {
        name: "Current download progress",
      }),
    ).not.toHaveAttribute("aria-valuenow");
  });

  it("shows file verification progress before download byte totals exist", async () => {
    backend([
      downloadJob({
        phase: "verifying",
        component: "modpack",
        progress: {
          finished: 3,
          failed: 0,
          total: 10,
          bytesDone: 0,
          bytesTotal: 0,
        },
      }),
    ]);
    renderDownloads();
    fireEvent.click(
      await screen.findByRole("button", { name: "Downloads: 30%" }),
    );
    const row = await screen.findByRole("region", {
      name: "Download for My Survival",
    });
    expect(within(row).getByText("Verifying · Modpack files")).toBeVisible();
    expect(within(row).getByText("30.0% · 3/10 files checked")).toBeVisible();
    expect(within(row).getByRole("progressbar")).toHaveAttribute(
      "aria-valuenow",
      "30",
    );
  });

  it.each([
    ["modpackarchive", "downloading", "Downloading · Modpack archive"],
    ["modpack", "downloading", "Downloading · Modpack files"],
    ["forgeprofile", "preparing", "Preparing · Forge profile"],
    ["neoforgeprofile", "preparing", "Preparing · NeoForge profile"],
    ["forge", "downloading", "Downloading · Forge"],
    ["forge", "installing", "Installing · Forge"],
  ] as const)("distinguishes %s during %s", async (component, phase, label) => {
    backend([downloadJob({ component, phase })]);
    renderDownloads();
    fireEvent.click(await screen.findByRole("button", { name: /^Downloads:/ }));
    const row = await screen.findByRole("region", {
      name: "Download for My Survival",
    });
    expect(within(row).getByText(label)).toBeVisible();
  });

  it("shows the instance, icon, component, percentage and working pause/resume/cancel controls", async () => {
    backend([downloadJob()]);
    renderDownloads();
    fireEvent.click(
      await screen.findByRole("button", { name: "Downloads: 25%" }),
    );
    const row = await screen.findByRole("region", {
      name: "Download for My Survival",
    });
    expect(within(row).getByText("Downloading · Minecraft")).toBeVisible();
    expect(
      within(row).getByRole("progressbar", {
        name: "Progress for My Survival",
      }),
    ).toHaveAttribute("aria-valuenow", "25");
    expect(row.querySelector("img")).toHaveAttribute(
      "src",
      "asset://localhost//icons/survival.png",
    );
    fireEvent.click(
      within(row).getByRole("button", {
        name: "Pause download for My Survival",
      }),
    );
    await screen.findByRole("button", { name: "Downloads: Paused" });
    expect(invoke).toHaveBeenCalledWith("pause_download_job", { jobId: "1" });
    fireEvent.click(
      within(row).getByRole("button", {
        name: "Resume download for My Survival",
      }),
    );
    await screen.findByRole("button", { name: "Downloads: 25%" });
    expect(invoke).toHaveBeenCalledWith("resume_download_job", { jobId: "1" });
    fireEvent.click(
      within(row).getByRole("button", {
        name: "Cancel download for My Survival",
      }),
    );
    const dialog = await screen.findByRole("dialog", {
      name: "Cancel download for My Survival?",
    });
    expect(dialog).toHaveTextContent(
      "Existing instances and their files will be kept.",
    );
    expect(invoke).not.toHaveBeenCalledWith("cancel_download_job", {
      jobId: "1",
    });
    fireEvent.click(
      within(dialog).getByRole("button", { name: "Keep downloading" }),
    );
    expect(
      screen.queryByRole("dialog", {
        name: "Cancel download for My Survival?",
      }),
    ).not.toBeInTheDocument();
    fireEvent.click(
      within(row).getByRole("button", {
        name: "Cancel download for My Survival",
      }),
    );
    fireEvent.click(
      within(
        await screen.findByRole("dialog", {
          name: "Cancel download for My Survival?",
        }),
      ).getByRole("button", { name: "Cancel download" }),
    );
    await screen.findByRole("button", { name: "Downloads: Idle" });
    expect(invoke).toHaveBeenCalledWith("cancel_download_job", { jobId: "1" });
    expect(
      screen.queryByRole("region", { name: "Download for My Survival" }),
    ).not.toBeInTheDocument();
  });

  it("offers manual recovery without starting interrupted tasks automatically", async () => {
    backend([downloadJob({ status: "interrupted", component: "fabric" })]);
    renderDownloads();
    fireEvent.click(
      await screen.findByRole("button", { name: "Downloads: Interrupted" }),
    );
    const resume = await screen.findByRole("button", {
      name: "Resume download for My Survival",
    });
    expect(
      invoke.mock.calls.some(([command]) => command === "resume_download_job"),
    ).toBe(false);
    expect(screen.getByText("Interrupted · Fabric")).toBeVisible();
    fireEvent.click(resume);
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("resume_download_job", {
        jobId: "1",
      }),
    );
  });

  it("uses an indeterminate bar and disables pause during extraction", async () => {
    backend([
      downloadJob({
        component: "java",
        phase: "extracting",
        progress: {
          finished: 0,
          failed: 0,
          total: 0,
          bytesDone: 0,
          bytesTotal: 0,
        },
      }),
    ]);
    renderDownloads();
    fireEvent.click(
      await screen.findByRole("button", { name: "Downloads: Extracting" }),
    );
    expect(await screen.findByText("Extracting · Java")).toBeVisible();
    expect(
      screen.getByRole("progressbar", { name: "Progress for My Survival" }),
    ).not.toHaveAttribute("aria-valuenow");
    expect(
      screen.getByRole("button", { name: "Pause download for My Survival" }),
    ).toBeDisabled();
    expect(
      screen.getByRole("button", { name: "Cancel download for My Survival" }),
    ).toBeEnabled();
  });

  it("keeps pending cancellation visible and confirms a cleanup retry without offering resume", async () => {
    backend([
      downloadJob({ status: "cancelling", error: "Instance file is locked" }),
    ]);
    renderDownloads();
    fireEvent.click(
      await screen.findByRole("button", {
        name: "Downloads: Cancellation pending",
      }),
    );
    const row = await screen.findByRole("region", {
      name: "Download for My Survival",
    });
    expect(within(row).getByRole("alert")).toHaveTextContent(
      "Instance file is locked",
    );
    expect(
      within(row).queryByRole("button", { name: /Resume|Pause/ }),
    ).not.toBeInTheDocument();
    fireEvent.click(
      within(row).getByRole("button", {
        name: "Retry cancellation for My Survival",
      }),
    );
    const dialog = await screen.findByRole("dialog", {
      name: "Retry cancellation for My Survival?",
    });
    expect(within(dialog).getByRole("alert")).toHaveTextContent(
      "Instance file is locked",
    );
    expect(invoke).not.toHaveBeenCalledWith("cancel_download_job", {
      jobId: "1",
    });
    fireEvent.click(
      within(dialog).getByRole("button", { name: "Retry cancellation" }),
    );
    await screen.findByRole("button", { name: "Downloads: Idle" });
    expect(invoke).toHaveBeenCalledWith("cancel_download_job", { jobId: "1" });
    expect(
      invoke.mock.calls.some(([command]) => command === "resume_download_job"),
    ).toBe(false);
  });

  it("shows global Java tasks without a fake instance", async () => {
    backend([
      downloadJob({ instanceFolder: null, javaMajor: 21, component: "java" }),
    ]);
    renderDownloads();
    fireEvent.click(
      await screen.findByRole("button", { name: "Downloads: 25%" }),
    );
    expect(
      await screen.findByRole("region", { name: "Download for Java 21" }),
    ).toBeVisible();
    expect(screen.queryByText("My Survival")).not.toBeInTheDocument();
    fireEvent.click(
      screen.getByRole("button", { name: "Cancel download for Java 21" }),
    );
    const dialog = await screen.findByRole("dialog", {
      name: "Cancel download for Java 21?",
    });
    expect(dialog).not.toHaveTextContent("removed from your library");
    expect(invoke).not.toHaveBeenCalledWith("cancel_download_job", {
      jobId: "1",
    });
    fireEvent.click(
      within(dialog).getByRole("button", { name: "Cancel download" }),
    );
    await screen.findByRole("button", { name: "Downloads: Idle" });
  });

  it("reports command errors and keeps the resumable task available", async () => {
    backend([
      downloadJob({ status: "failed", error: "Checksum verification failed" }),
    ]);
    renderDownloads();
    fireEvent.click(
      await screen.findByRole("button", { name: "Downloads: Failed" }),
    );
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Checksum verification failed",
    );
    invoke.mockRejectedValueOnce(new Error("network unavailable"));
    fireEvent.click(
      screen.getByRole("button", { name: "Resume download for My Survival" }),
    );
    await waitFor(() => expect(toast.error).toHaveBeenCalled());
    expect(
      screen.getByRole("region", { name: "Download for My Survival" }),
    ).toBeVisible();
  });
});
