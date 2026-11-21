import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { startupProgress } from "@/features/startup/fixtures/progress";
import { StartupGate } from "@/features/startup/startup-gate";
import { StartupScreen } from "@/features/startup/startup-screen";
import type { StartupProgress } from "@/lib/generated/StartupProgress";

const { getProgress, listen, listeners } = vi.hoisted(() => ({
  getProgress: vi.fn(),
  listen: vi.fn(),
  listeners: new Set<(event: { payload: StartupProgress }) => void>(),
}));
vi.mock("@/api/commands", () => ({ getStartupProgress: getProgress }));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    label: "main",
    close: vi.fn(),
    minimize: vi.fn(),
    show: vi.fn().mockResolvedValue(undefined),
  }),
}));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

function view() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <StartupGate fallback={<StartupScreen />}>
        <p>Launcher interface</p>
      </StartupGate>
    </QueryClientProvider>,
  );
}

describe("StartupGate", () => {
  beforeEach(() => {
    listeners.clear();
    getProgress.mockReset();
    listen
      .mockReset()
      .mockImplementation(
        async (
          _name: string,
          handler: (event: { payload: StartupProgress }) => void,
        ) => {
          listeners.add(handler);
          return () => listeners.delete(handler);
        },
      );
  });

  it("shows startup progress in the launcher window, then the launcher without opening another window", async () => {
    getProgress.mockResolvedValue(
      startupProgress({ phase: "downloadingUpdate", version: "1.0" }),
    );
    view();
    expect(await screen.findByText("Downloading update…")).toBeVisible();
    expect(screen.queryByText("Launcher interface")).not.toBeInTheDocument();
    act(() => {
      for (const handler of listeners)
        handler({ payload: startupProgress({ phase: "ready" }) });
    });
    expect(await screen.findByText("Launcher interface")).toBeVisible();
    expect(screen.queryByText("Downloading update…")).not.toBeInTheDocument();
  });

  it("keeps a startup failure on screen instead of opening the launcher", async () => {
    getProgress.mockResolvedValue(
      startupProgress({ phase: "failed", error: "Could not prepare storage" }),
    );
    view();
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Could not prepare storage",
    );
    expect(screen.queryByText("Launcher interface")).not.toBeInTheDocument();
  });

  it("opens the launcher directly when the window reloads after startup", async () => {
    getProgress.mockResolvedValue(startupProgress({ phase: "ready" }));
    view();
    expect(await screen.findByText("Launcher interface")).toBeVisible();
  });
});
