import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { StrictMode } from "react";
import { afterAll, beforeEach, describe, expect, it, vi } from "vitest";
import { startupProgress } from "@/features/startup/fixtures/progress";
import { StartupScreen } from "@/features/startup/startup-screen";
import type { StartupProgress } from "@/lib/generated/StartupProgress";

const {
  getProgress,
  center,
  show,
  close,
  windowInfo,
  decode,
  listen,
  listeners,
} = vi.hoisted(() => ({
  getProgress: vi.fn(),
  center: vi.fn(),
  show: vi.fn(),
  close: vi.fn(),
  windowInfo: { label: "startup" },
  decode: vi.fn(),
  listen: vi.fn(),
  listeners: new Set<(event: { payload: StartupProgress }) => void>(),
}));
vi.mock("@/lib/commands", () => ({ getStartupProgress: getProgress }));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ center, show, close, label: windowInfo.label }),
}));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

const originalDecode = Object.getOwnPropertyDescriptor(
  HTMLImageElement.prototype,
  "decode",
);
Object.defineProperty(HTMLImageElement.prototype, "decode", {
  configurable: true,
  value: decode,
});
afterAll(() => {
  if (originalDecode) {
    Object.defineProperty(HTMLImageElement.prototype, "decode", originalDecode);
  } else {
    Reflect.deleteProperty(HTMLImageElement.prototype, "decode");
  }
});

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
    center.mockReset();
    show.mockReset();
    close.mockReset().mockResolvedValue(undefined);
    windowInfo.label = "startup";
    decode.mockReset();
    listen.mockReset();
    center.mockResolvedValue(undefined);
    show.mockResolvedValue(undefined);
    decode.mockResolvedValue(undefined);
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

  it("does not show an X button inside the scenario preview", async () => {
    windowInfo.label = "startup-preview-1";
    getProgress.mockResolvedValue(startupProgress({ preview: true }));
    view();
    await screen.findByText(/Development preview/);
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("closes a separate preview with Escape while it is loading", async () => {
    windowInfo.label = "startup-preview-2";
    view();
    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => expect(close).toHaveBeenCalledOnce());
  });

  it("does not expose preview close controls or Escape on the real startup window", async () => {
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

  it("keeps a migration failure visible without a close button", async () => {
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
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
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

  it("renders the existing logo above the progress and status on the sidebar background", async () => {
    getProgress.mockResolvedValue(
      startupProgress({
        phase: "migratingInstances",
        processedFiles: 10,
        processedBytes: 1048576,
      }),
    );
    view();
    await screen.findByText("Migrating instances…");
    const logo = screen.getByRole("img", { name: "Licht Launcher" });
    const progress = screen.getByRole("progressbar");
    const counter = screen.getByText("10 files verified · 1.0 MB");
    const status = screen.getByRole("status");
    expect(logo.getAttribute("src")).toContain("128x128@2x.png");
    expect(logo.getAttribute("src")).not.toMatch(/^data:/);
    expect(
      logo.compareDocumentPosition(progress) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(
      progress.compareDocumentPosition(counter) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(
      counter.compareDocumentPosition(status) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(logo.closest("[data-tauri-drag-region]")).not.toBeNull();
    expect(screen.getByRole("main")).toHaveClass(
      "bg-sidebar",
      "text-sidebar-foreground",
    );
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("waits for the logo, centers and shows the rendered interface before starting the check", async () => {
    let finishDecode: (() => void) | undefined;
    decode.mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          finishDecode = resolve;
        }),
    );
    center.mockImplementation(async () => {
      expect(screen.getByRole("main")).toHaveClass("bg-sidebar");
      expect(screen.getByRole("status")).toHaveTextContent(
        "Preparing launcher…",
      );
      expect(show).not.toHaveBeenCalled();
    });
    show.mockImplementation(async () => {
      expect(center).toHaveBeenCalledTimes(1);
      expect(getProgress).not.toHaveBeenCalled();
    });
    view();
    await waitFor(() => expect(decode).toHaveBeenCalledTimes(1));
    expect(center).not.toHaveBeenCalled();
    expect(show).not.toHaveBeenCalled();
    expect(getProgress).not.toHaveBeenCalled();
    await act(async () => finishDecode?.());
    await waitFor(() => expect(getProgress).toHaveBeenCalledTimes(1));
    expect(show).toHaveBeenCalledTimes(1);
  });

  it("still reveals the interface and reports a logo loading failure", async () => {
    decode.mockRejectedValue(new Error("Logo decoding failed"));
    view();
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Logo decoding failed",
    );
    expect(center).toHaveBeenCalledTimes(1);
    expect(show).toHaveBeenCalledTimes(1);
  });
});
