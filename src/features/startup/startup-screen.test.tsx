import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { startupProgress } from "@/features/startup/fixtures/progress";
import { StartupScreen } from "@/features/startup/startup-screen";
import type { StartupProgress } from "@/lib/generated/StartupProgress";

const { getProgress, close, listen, listeners } = vi.hoisted(() => ({
  getProgress: vi.fn(),
  close: vi.fn(),
  listen: vi.fn(),
  listeners: new Set<(event: { payload: StartupProgress }) => void>(),
}));
vi.mock("@/lib/commands", () => ({ getStartupProgress: getProgress }));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ close }),
}));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

function view() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const result = render(
    <QueryClientProvider client={client}>
      <StartupScreen />
    </QueryClientProvider>,
  );
  return { ...result, client };
}

describe("StartupScreen", () => {
  beforeEach(() => {
    listeners.clear();
    getProgress.mockReset();
    close.mockReset();
    listen.mockReset();
    close.mockResolvedValue(undefined);
    getProgress.mockResolvedValue(startupProgress());
    listen.mockImplementation(
      async (
        _name: string,
        handler: (event: { payload: StartupProgress }) => void,
      ) => {
        listeners.add(handler);
        return () => {
          listeners.delete(handler);
        };
      },
    );
  });

  it("shows verified migration work without inventing a percentage", async () => {
    getProgress.mockResolvedValue(
      startupProgress({
        phase: "migratingInstances",
        processedFiles: 23,
        processedBytes: 1048576,
      }),
    );
    view();
    expect(await screen.findByText("Migrating instances…")).toBeVisible();
    expect(screen.getByText("23 files verified · 1.0 MB")).toBeVisible();
    expect(screen.getByRole("progressbar")).not.toHaveAttribute(
      "aria-valuenow",
    );
  });

  it("uses download byte totals and shows the update version", async () => {
    getProgress.mockResolvedValue(
      startupProgress({
        phase: "downloadingUpdate",
        processedBytes: 1048576,
        totalBytes: 2097152,
        version: "1.0",
        preview: true,
      }),
    );
    view();
    expect(await screen.findByText("Downloading update…")).toBeVisible();
    expect(screen.getByRole("progressbar")).toHaveAttribute(
      "aria-valuenow",
      "50",
    );
    expect(screen.getByText("Update: 1.0")).toBeVisible();
    expect(screen.getByText(/Development preview/)).toBeVisible();
  });

  it("preserves a newer event received while the initial snapshot is loading and cleans up", async () => {
    let finish: ((snapshot: StartupProgress) => void) | undefined;
    getProgress.mockImplementation(
      () =>
        new Promise<StartupProgress>((resolve) => {
          finish = resolve;
        }),
    );
    const { unmount } = view();
    await waitFor(() => expect(getProgress).toHaveBeenCalledTimes(1));
    act(() => {
      for (const handler of listeners)
        handler({
          payload: startupProgress({
            phase: "migratingData",
            processedFiles: 10,
          }),
        });
    });
    expect(
      await screen.findByText("Migrating shared game files…"),
    ).toBeVisible();
    await act(async () => finish?.(startupProgress({ phase: "preparing" })));
    expect(screen.getByText("Migrating shared game files…")).toBeVisible();
    unmount();
    expect(listeners.size).toBe(0);
  });

  it("keeps a migration failure visible and lets the user close the window", async () => {
    getProgress.mockResolvedValue(
      startupProgress({
        phase: "failed",
        error: "Destination contains different data",
      }),
    );
    view();
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Destination contains different data",
    );
    expect(screen.queryByRole("progressbar")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    await waitFor(() => expect(close).toHaveBeenCalledTimes(1));
  });

  it("falls back to polling when live events are unavailable and displays offline warnings", async () => {
    listen.mockRejectedValueOnce(new Error("bridge unavailable"));
    getProgress.mockResolvedValue(
      startupProgress({ phase: "ready", warning: "Starting offline" }),
    );
    view();
    expect(await screen.findByText("Starting offline")).toBeVisible();
    expect(
      screen.getByText("Live progress is unavailable. Polling startup status."),
    ).toBeVisible();
    expect(screen.getByRole("status")).toHaveTextContent("Ready");
  });
});
