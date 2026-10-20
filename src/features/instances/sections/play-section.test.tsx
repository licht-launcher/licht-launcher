import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { downloadJob } from "@/features/downloads/fixtures/job";
import { PlaySection } from "@/features/instances/sections/play-section";
import type { DownloadJobSnapshot } from "@/lib/generated/DownloadJobSnapshot";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

const instance: InstanceEntry = {
  folder: "classic",
  name: "Classic",
  versionId: "1.5.2",
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
  lastOpenedAt: new Date(Date.now() - 5 * 60_000).toISOString(),
  iconPath: null,
};

function renderJob(job: DownloadJobSnapshot, speedBytesPerSecond = 0) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const section = (current: DownloadJobSnapshot, speed: number) => (
    <QueryClientProvider client={client}>
      <PlaySection
        instance={instance}
        busy={false}
        playDisabled={false}
        phase="idle"
        installing={current.status === "running"}
        downloadJob={current}
        progress={current.progress}
        speedBytesPerSecond={speed}
        onPlay={vi.fn()}
        onStop={vi.fn()}
        onConfigure={vi.fn()}
      />
    </QueryClientProvider>
  );
  const view = render(section(job, speedBytesPerSecond));
  return {
    updateJob: (next: DownloadJobSnapshot, speed = speedBytesPerSecond) =>
      view.rerender(section(next, speed)),
  };
}

describe("PlaySection", () => {
  it("shows the actual downloading phase when the managed job has no speed sample", () => {
    renderJob(downloadJob());

    expect(screen.getByText("Downloading · Minecraft")).toBeVisible();
    expect(
      screen.queryByText("Preparing game files and modpack content…"),
    ).not.toBeInTheDocument();
  });

  it.each([
    ["preparing", "Preparing"],
    ["downloading", "Downloading"],
    ["verifying", "Verifying"],
    ["extracting", "Extracting"],
    ["installing", "Installing"],
  ] as const)(
    "shows the %s job phase independently of speed",
    (phase, label) => {
      renderJob(downloadJob({ phase }));

      expect(screen.getByText(`${label} · Minecraft`)).toBeVisible();
    },
  );

  it.each([
    ["queued", "Queued"],
    ["paused", "Paused"],
    ["interrupted", "Interrupted"],
    ["cancelling", "Cancellation pending"],
    ["failed", "Failed"],
    ["completed", "Completed"],
    ["cancelled", "Cancelled"],
  ] as const)(
    "prioritizes %s over a previous downloading phase and speed",
    (status, label) => {
      renderJob(downloadJob({ status }), 1024 * 1024);

      expect(screen.getByText(`${label} · Minecraft`)).toBeVisible();
      expect(screen.queryByText(/MB\/s/)).not.toBeInTheDocument();
    },
  );

  it("updates the status as the job changes phase and component", () => {
    const { updateJob } = renderJob(downloadJob({ phase: "verifying" }));
    expect(screen.getByText("Verifying · Minecraft")).toBeVisible();

    updateJob(downloadJob({ component: "java", phase: "extracting" }));
    expect(screen.getByText("Extracting · Java")).toBeVisible();
    expect(screen.queryByText("Verifying · Minecraft")).not.toBeInTheDocument();

    updateJob(downloadJob({ component: "fabric", phase: "installing" }));
    expect(screen.getByText("Installing · Fabric")).toBeVisible();
    expect(screen.queryByText("Extracting · Java")).not.toBeInTheDocument();
  });

  it("shows a status without a component when the component is not resolved", () => {
    renderJob(downloadJob({ component: null }));

    expect(screen.getByText("Downloading")).toBeVisible();
  });

  it("appends a positive speed only while the job is downloading", () => {
    const { updateJob } = renderJob(downloadJob(), 1024 * 1024);
    expect(
      screen.getByText("Downloading · Minecraft · 1.0 MB/s"),
    ).toBeVisible();

    updateJob(downloadJob({ phase: "verifying" }));
    expect(screen.getByText("Verifying · Minecraft")).toBeVisible();
    expect(screen.queryByText(/MB\/s/)).not.toBeInTheDocument();
  });

  it("shows tooltips for version, loader, and last played", async () => {
    render(
      <PlaySection
        instance={instance}
        busy={false}
        playDisabled={false}
        phase="idle"
        installing={false}
        progress={null}
        onPlay={vi.fn()}
        onStop={vi.fn()}
        onConfigure={vi.fn()}
      />,
    );

    fireEvent.focus(screen.getByRole("button", { name: /1\.5\.2/ }));
    expect(await screen.findByRole("tooltip")).toHaveTextContent(
      "Game version",
    );

    fireEvent.blur(screen.getByRole("button", { name: /1\.5\.2/ }));
    fireEvent.focus(screen.getByRole("button", { name: /Vanilla/ }));
    expect(await screen.findByRole("tooltip")).toHaveTextContent("Loader");

    fireEvent.blur(screen.getByRole("button", { name: /Vanilla/ }));
    fireEvent.focus(screen.getByRole("button", { name: /minutes ago/ }));
    expect(await screen.findByRole("tooltip")).toHaveTextContent("Last played");
  });

  it("places install progress below the title row", () => {
    const { container } = render(
      <PlaySection
        instance={instance}
        busy={false}
        playDisabled={false}
        phase="idle"
        installing
        progress={{
          finished: 1,
          failed: 0,
          total: 10,
          bytesDone: 100,
          bytesTotal: 1000,
        }}
        onPlay={vi.fn()}
        onStop={vi.fn()}
        onConfigure={vi.fn()}
      />,
    );

    const root = container.firstElementChild;
    expect(root).toHaveClass("flex-col");
    expect(screen.getByRole("progressbar", { name: "Install progress" })).toBe(
      root?.lastElementChild?.querySelector('[role="progressbar"]'),
    );
    expect(screen.getByRole("button", { name: "Play" })).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Configure Classic" }),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Preparing game files and modpack content…"),
    ).toBeVisible();
  });
});
