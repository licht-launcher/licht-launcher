import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { StrictMode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { startupProgress } from "@/features/startup/fixtures/progress";
import { StartupScreen } from "@/features/startup/startup-screen";
import type { StartupProgress } from "@/lib/generated/StartupProgress";

const { getProgress, close, minimize, show, windowInfo, listen, listeners } =
  vi.hoisted(() => ({
    getProgress: vi.fn(),
    close: vi.fn(),
    minimize: vi.fn(),
    show: vi.fn(),
    windowInfo: { label: "main" },
    listen: vi.fn(),
    listeners: new Set<(event: { payload: StartupProgress }) => void>(),
  }));
vi.mock("@/api/commands", () => ({ getStartupProgress: getProgress }));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ close, minimize, show, label: windowInfo.label }),
}));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

function view() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const result = render(
    <StrictMode>
      <QueryClientProvider client={client}>
        <StartupScreen />
      </QueryClientProvider>
    </StrictMode>,
  );
  return { ...result, client };
}

describe("StartupScreen", () => {
  beforeEach(() => {
    listeners.clear();
    getProgress.mockReset();
    close.mockReset().mockResolvedValue(undefined);
    minimize.mockReset().mockResolvedValue(undefined);
    show.mockReset().mockResolvedValue(undefined);
    windowInfo.label = "main";
    listen.mockReset();
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

  it("shows the removal of the migrated legacy profile", async () => {
    getProgress.mockResolvedValue(
      startupProgress({
        phase: "cleaningLegacyData",
        processedFiles: 4,
        processedBytes: 2097152,
      }),
    );
    view();
    expect(
      await screen.findByText("Removing old launcher files…"),
    ).toBeVisible();
    expect(screen.getByText("4 files verified · 2.0 MB")).toBeVisible();
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

  it("does not show a separate close-preview button inside the scenario preview", async () => {
    windowInfo.label = "startup-preview-1";
    getProgress.mockResolvedValue(startupProgress({ preview: true }));
    view();
    await screen.findByText(/Development preview/);
    expect(
      screen.queryByRole("button", { name: "Close preview" }),
    ).not.toBeInTheDocument();
  });

  it("closes a separate preview with Escape while it is loading", async () => {
    windowInfo.label = "startup-preview-2";
    view();
    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => expect(close).toHaveBeenCalledOnce());
  });

  it("does not close the real launcher window with Escape", async () => {
    getProgress.mockResolvedValue(startupProgress({ preview: true }));
    view();
    await screen.findByText(/Development preview/);
    expect(
      screen.queryByRole("button", { name: "Close preview" }),
    ).not.toBeInTheDocument();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(close).not.toHaveBeenCalled();
  });

  it("reports close failures and lets the preview close be retried", async () => {
    windowInfo.label = "startup-preview-3";
    close.mockRejectedValueOnce(new Error("Close permission unavailable"));
    view();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Could not close preview: Close permission unavailable",
    );
    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => expect(close).toHaveBeenCalledTimes(2));
    await waitFor(() =>
      expect(screen.queryByRole("alert")).not.toBeInTheDocument(),
    );
  });

  it("prevents another close request while the current one is pending", async () => {
    let finishClose: (() => void) | undefined;
    close.mockReturnValueOnce(
      new Promise<void>((resolve) => {
        finishClose = resolve;
      }),
    );
    windowInfo.label = "startup-preview-4";
    view();
    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() =>
      expect(screen.getByRole("main")).toHaveAttribute("aria-busy", "true"),
    );
    fireEvent.keyDown(window, { key: "Escape" });
    expect(close).toHaveBeenCalledOnce();
    await act(async () => finishClose?.());
    await waitFor(() =>
      expect(screen.getByRole("main")).toHaveAttribute("aria-busy", "false"),
    );
  });

  it("keeps a migration failure visible and lets the window be closed", async () => {
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
    await waitFor(() => expect(close).toHaveBeenCalledOnce());
  });

  it("falls back to polling when live events are unavailable and displays offline warnings", async () => {
    listen.mockRejectedValue(new Error("bridge unavailable"));
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

  it("renders the launcher title bar with window controls and no installer logo", async () => {
    getProgress.mockResolvedValue(
      startupProgress({
        phase: "migratingInstances",
        processedFiles: 10,
        processedBytes: 1048576,
      }),
    );
    view();
    await screen.findByText("Migrating instances…");
    const titleBar = screen.getByRole("banner");
    expect(titleBar).toHaveTextContent("Licht Launcher");
    expect(titleBar.querySelector("[data-tauri-drag-region]")).not.toBeNull();
    expect(screen.queryByRole("img")).not.toBeInTheDocument();
    const status = screen.getByRole("status");
    const progress = screen.getByRole("progressbar");
    const counter = screen.getByText("10 files verified · 1.0 MB");
    expect(
      status.compareDocumentPosition(progress) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(
      progress.compareDocumentPosition(counter) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Minimize" }));
    await waitFor(() => expect(minimize).toHaveBeenCalledOnce());
  });

  it("shows the hidden window once rendered, then starts reading progress", async () => {
    let finishShow: (() => void) | undefined;
    show.mockImplementation(() => {
      expect(screen.getByRole("status")).toHaveTextContent(
        "Preparing launcher…",
      );
      return new Promise<void>((resolve) => {
        finishShow = resolve;
      });
    });
    getProgress.mockResolvedValue(startupProgress({ phase: "preparing" }));
    view();
    await waitFor(() => expect(show).toHaveBeenCalledOnce());
    expect(getProgress).not.toHaveBeenCalled();
    await act(async () => finishShow?.());
    await waitFor(() => expect(getProgress).toHaveBeenCalledTimes(1));
  });

  it("still reads progress and reports it when the window cannot be shown", async () => {
    show.mockRejectedValue(new Error("Show permission unavailable"));
    getProgress.mockResolvedValue(startupProgress({ phase: "preparing" }));
    view();
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Could not show the launcher window: Show permission unavailable",
    );
    await waitFor(() => expect(getProgress).toHaveBeenCalledTimes(1));
  });
});
