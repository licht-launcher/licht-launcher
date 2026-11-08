import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { TitleBar } from "@/components/title-bar/title-bar";
import { DownloadJobsProvider } from "@/features/downloads/components/download-jobs-provider";
import { snapshot } from "@/features/settings/fixtures/snapshot";
import {
  INSTALL_UPDATE_EVENT,
  UPDATE_CHECK_STATUS_EVENT,
} from "@/features/updater/components/update-check";
import * as nativeCaption from "@/lib/native-caption";

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
      <DownloadJobsProvider>
        <TitleBar />
      </DownloadJobsProvider>
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
      if (command === "list_download_jobs" || command === "list_instances")
        return [];
      throw new Error(`unexpected command: ${command}`);
    });
  });

  it("shows a drag region and window controls without duplicating account or download shortcuts", () => {
    vi.spyOn(nativeCaption, "usesNativeCaptionButtons").mockReturnValue(false);
    renderTitleBar();

    expect(
      document.querySelector("[data-tauri-drag-region]"),
    ).toBeInTheDocument();
    expect(screen.queryByText("Licht Launcher")).not.toBeInTheDocument();

    expect(
      screen.queryByRole("button", { name: /^Account:/ }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /^Downloads:/ }),
    ).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Minimize" }));
    expect(minimize).toHaveBeenCalledOnce();

    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    expect(close).toHaveBeenCalledOnce();
  });

  it("omits HTML window controls when native caption buttons are used", () => {
    vi.spyOn(nativeCaption, "usesNativeCaptionButtons").mockReturnValue(true);
    renderTitleBar();

    expect(
      screen.queryByRole("button", { name: "Minimize" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Close" }),
    ).not.toBeInTheDocument();
  });

  it("shows the update action and requests update installation", async () => {
    renderTitleBar();

    sessionStorage.setItem("licht-launcher.update-check-status", "available");
    window.dispatchEvent(
      new CustomEvent(UPDATE_CHECK_STATUS_EVENT, {
        detail: { status: "available", checking: false },
      }),
    );

    const update = await screen.findByRole("button", { name: "Update" });

    const install = vi.fn();
    window.addEventListener(INSTALL_UPDATE_EVENT, install);
    fireEvent.click(update);
    expect(install).toHaveBeenCalledOnce();
    window.removeEventListener(INSTALL_UPDATE_EVENT, install);
  });
});
