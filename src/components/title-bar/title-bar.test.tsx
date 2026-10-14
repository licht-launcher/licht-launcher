import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { TitleBar } from "@/components/title-bar/title-bar";
import { snapshot } from "@/features/settings/fixtures/snapshot";
import {
  INSTALL_UPDATE_EVENT,
  UPDATE_CHECK_STATUS_EVENT,
} from "@/features/updater/components/update-check";
import type { InstallProgress } from "@/lib/generated/InstallProgress";
import * as nativeCaption from "@/lib/native-caption";
import { OPEN_SETTINGS_EVENT } from "@/lib/shell-events";

const minimize = vi.fn();
const close = vi.fn();

const { invoke, listeners } = vi.hoisted(() => ({
  invoke: vi.fn(),
  listeners: new Map<string, (event: { payload: unknown }) => void>(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke,
}));

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

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    minimize,
    close,
    setDecorations: vi.fn(),
  }),
}));

function renderTitleBar() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <TitleBar />
    </QueryClientProvider>,
  );
}

describe("TitleBar", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    minimize.mockReset();
    close.mockReset();
    invoke.mockReset();
    listeners.clear();
    sessionStorage.clear();
    invoke.mockImplementation(async (command: string) => {
      if (command === "get_settings") {
        return {
          ...snapshot,
          settings: { ...snapshot.settings, username: "Alex" },
        };
      }
      throw new Error(`unexpected command: ${command}`);
    });
  });

  it("shows a drag region, account chip, idle downloads, and window controls", async () => {
    vi.spyOn(nativeCaption, "usesNativeCaptionButtons").mockReturnValue(false);
    renderTitleBar();

    expect(
      document.querySelector("[data-tauri-drag-region]"),
    ).toBeInTheDocument();
    expect(screen.queryByText("Licht Launcher")).not.toBeInTheDocument();

    expect(
      await screen.findByRole("button", { name: "Account: Alex" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Downloads: Idle" }),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Minimize" }));
    expect(minimize).toHaveBeenCalledOnce();

    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    expect(close).toHaveBeenCalledOnce();
  });

  it("omits HTML window controls when native caption buttons are used", async () => {
    vi.spyOn(nativeCaption, "usesNativeCaptionButtons").mockReturnValue(true);
    renderTitleBar();

    expect(
      await screen.findByRole("button", { name: "Account: Alex" }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Minimize" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Close" }),
    ).not.toBeInTheDocument();
  });

  it("requests account settings when the account chip is clicked", async () => {
    renderTitleBar();
    const onOpen = vi.fn();
    window.addEventListener(OPEN_SETTINGS_EVENT, onOpen);

    const account = await screen.findByRole("button", {
      name: "Account: Alex",
    });
    fireEvent.pointerDown(account, { button: 0 });

    expect(onOpen).toHaveBeenCalledOnce();
    const event = onOpen.mock.calls[0]?.[0] as CustomEvent | undefined;
    expect(event?.detail).toEqual({ section: "account" });

    window.removeEventListener(OPEN_SETTINGS_EVENT, onOpen);
  });

  it("shows the update action before the account chip", async () => {
    renderTitleBar();
    await screen.findByRole("button", { name: "Account: Alex" });

    sessionStorage.setItem("licht-launcher.update-check-status", "available");
    window.dispatchEvent(
      new CustomEvent(UPDATE_CHECK_STATUS_EVENT, {
        detail: { status: "available", checking: false },
      }),
    );

    const update = await screen.findByRole("button", { name: "Update" });
    expect(
      update.compareDocumentPosition(
        screen.getByRole("button", { name: "Account: Alex" }),
      ) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();

    const install = vi.fn();
    window.addEventListener(INSTALL_UPDATE_EVENT, install);
    fireEvent.click(update);
    expect(install).toHaveBeenCalledOnce();
    window.removeEventListener(INSTALL_UPDATE_EVENT, install);
  });

  it("shows download progress on the chip and in the popover", async () => {
    renderTitleBar();
    await screen.findByRole("button", { name: "Downloads: Idle" });

    await waitFor(() => {
      expect(listeners.has("install-progress")).toBe(true);
    });

    listeners.get("install-progress")?.({
      payload: {
        versionId: "1.20.1",
        finished: 1,
        failed: 0,
        total: 4,
        bytesDone: 100_000,
        bytesTotal: 400_000,
      } satisfies InstallProgress,
    });

    expect(
      await screen.findByRole("button", { name: "Downloads: 25%" }),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Downloads: 25%" }));
    expect(await screen.findByText(/25\.0%/)).toBeInTheDocument();

    listeners.get("install-progress")?.({
      payload: {
        versionId: "1.20.1",
        finished: 4,
        failed: 0,
        total: 4,
        bytesDone: 400_000,
        bytesTotal: 400_000,
      } satisfies InstallProgress,
    });

    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: "Downloads: Idle" }),
      ).toBeInTheDocument();
    });
  });

  it("shows idle detail in the downloads popover", async () => {
    renderTitleBar();
    fireEvent.click(
      await screen.findByRole("button", { name: "Downloads: Idle" }),
    );
    expect(
      await screen.findByText("No downloads in progress"),
    ).toBeInTheDocument();
  });

  it("stays Idle on a zero-total preparing pulse", async () => {
    renderTitleBar();
    await waitFor(() => {
      expect(listeners.has("install-progress")).toBe(true);
    });

    listeners.get("install-progress")?.({
      payload: {
        versionId: "fabric-loader-0.16.0-1.20.1",
        finished: 0,
        failed: 0,
        total: 0,
        bytesDone: 0,
        bytesTotal: 0,
      } satisfies InstallProgress,
    });

    expect(
      screen.getByRole("button", { name: "Downloads: Idle" }),
    ).toBeInTheDocument();
  });

  it("aggregates multiple installs on the downloads chip", async () => {
    renderTitleBar();
    await waitFor(() => {
      expect(listeners.has("install-progress")).toBe(true);
    });

    listeners.get("install-progress")?.({
      payload: {
        versionId: "1.20.1",
        finished: 1,
        failed: 0,
        total: 4,
        bytesDone: 100_000,
        bytesTotal: 400_000,
      } satisfies InstallProgress,
    });
    listeners.get("install-progress")?.({
      payload: {
        versionId: "1.21.1",
        finished: 0,
        failed: 0,
        total: 2,
        bytesDone: 0,
        bytesTotal: 200_000,
      } satisfies InstallProgress,
    });

    expect(
      await screen.findByRole("button", { name: "Downloads: 2" }),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Downloads: 2" }));
    expect(await screen.findByText("2 installing")).toBeInTheDocument();
  });
});
