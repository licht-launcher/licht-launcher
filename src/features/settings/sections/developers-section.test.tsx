import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { STORAGE_LOADING_PREVIEW_EVENT } from "@/features/downloads/hooks/use-storage-loading-preview";
import { snapshot } from "@/features/settings/fixtures/snapshot";
import { type Draft, fromSnapshot } from "@/features/settings/model/draft";
import { DevelopersSection } from "@/features/settings/sections/developers-section";
import {
  DEV_CLEAR_SIMULATED_UPDATE_EVENT,
  DEV_SIMULATE_UPDATE_EVENT,
} from "@/features/updater/components/update-check";

const {
  openPreview,
  closePreview,
  getPreviewOpen,
  previewWindow,
  previewListeners,
  restartPreview,
  resizeWindow,
  resetWindow,
} = vi.hoisted(() => ({
  openPreview: vi.fn(),
  closePreview: vi.fn(),
  getPreviewOpen: vi.fn(),
  previewWindow: { open: false },
  previewListeners: new Set<(event: { payload: unknown }) => void>(),
  restartPreview: vi.fn(),
  resizeWindow: vi.fn(),
  resetWindow: vi.fn(),
}));
vi.mock("@/lib/commands", () => ({
  openStartupPreview: openPreview,
  closeStartupPreview: closePreview,
  isStartupPreviewOpen: getPreviewOpen,
  restartStartupPreview: restartPreview,
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(
    async (_name: string, handler: (event: { payload: unknown }) => void) => {
      previewListeners.add(handler);
      return () => {
        previewListeners.delete(handler);
      };
    },
  ),
}));
vi.mock("@/lib/window-preview", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/lib/window-preview")>()),
  previewWindowSize: resizeWindow,
  resetWindowBounds: resetWindow,
}));

function view(
  onChange = vi.fn(),
  patch: Partial<Draft> = {},
  client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  }),
) {
  return render(
    <QueryClientProvider client={client}>
      <DevelopersSection
        draft={{ ...fromSnapshot(snapshot, null), ...patch }}
        onChange={onChange}
      />
    </QueryClientProvider>,
  );
}

describe("DevelopersSection", () => {
  beforeEach(() => {
    previewListeners.clear();
    previewWindow.open = false;
    getPreviewOpen
      .mockReset()
      .mockImplementation(async () => previewWindow.open);
    openPreview.mockReset().mockImplementation(async () => {
      previewWindow.open = true;
      for (const handler of previewListeners) handler({ payload: true });
    });
    closePreview.mockReset().mockImplementation(async () => {
      previewWindow.open = false;
      for (const handler of previewListeners) handler({ payload: false });
    });
    restartPreview.mockReset();
    resizeWindow.mockReset().mockResolvedValue(undefined);
    resetWindow.mockReset().mockResolvedValue(undefined);
    sessionStorage.clear();
  });

  it("shows the offline username directly and sends edits through the settings draft", () => {
    const onChange = vi.fn();
    view(onChange);
    expect(
      screen.getByRole("region", { name: "Developer Tools" }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Offline username" }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("textbox", { name: "Offline username" }),
    ).toHaveValue("Steve");
    fireEvent.change(
      screen.getByRole("textbox", { name: "Offline username" }),
      { target: { value: "Builder" } },
    );
    expect(onChange).toHaveBeenCalledWith({ username: "Builder" });
  });

  it("groups window and concurrency controls and forwards their changes", () => {
    const onChange = vi.fn();
    view(onChange);
    fireEvent.click(screen.getByRole("switch", { name: "System title bar" }));
    fireEvent.keyDown(
      screen.getByRole("slider", { name: "Simultaneous downloads" }),
      { key: "ArrowRight" },
    );
    fireEvent.keyDown(
      screen.getByRole("slider", { name: "Simultaneous writes" }),
      { key: "ArrowLeft" },
    );
    expect(onChange).toHaveBeenCalledWith({ disableCustomTitleBar: true });
    expect(onChange).toHaveBeenCalledWith({ downloadConcurrency: 9 });
    expect(onChange).toHaveBeenCalledWith({ writeConcurrency: 7 });
    expect(
      screen
        .getAllByRole("heading", { level: 3 })
        .map((heading) => heading.textContent),
    ).toEqual(["Player", "Window", "Transfers", "Testing"]);
    const window = within(screen.getByRole("region", { name: "Window" }));
    expect(
      window.getByRole("switch", { name: "System title bar" }),
    ).toBeInTheDocument();
    expect(
      window.getByRole("button", { name: "Reset window bounds" }),
    ).toBeInTheDocument();
    const testing = within(screen.getByRole("region", { name: "Testing" }));
    expect(
      screen.queryByRole("region", { name: /Acrylic/i }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("switch", { name: /Acrylic/i }),
    ).not.toBeInTheDocument();
    expect(
      testing.getByRole("switch", { name: "Simulate available update" }),
    ).toBeInTheDocument();
    expect(
      testing.getByRole("switch", { name: "Simulate storage loading" }),
    ).toBeInTheDocument();
    expect(
      testing.getByRole("combobox", { name: "Startup test scenario" }),
    ).toBeInTheDocument();
    expect(
      testing.getByRole("button", { name: "Open preview" }),
    ).toBeInTheDocument();
    expect(
      testing.getByRole("button", { name: "Restart with preview" }),
    ).toBeInTheDocument();
  });

  it.each([
    ["Small (960 × 560)", "small"],
    ["Default (1200 × 800)", "default"],
    ["Large (1200 × 960)", "large"],
  ])("applies the %s window preset", async (label, preset) => {
    view();
    const window = within(screen.getByRole("region", { name: "Window" }));
    fireEvent.click(window.getByRole("button", { name: label }));
    await waitFor(() => expect(resizeWindow).toHaveBeenCalledWith(preset));
    expect(resetWindow).not.toHaveBeenCalled();
  });

  it("restores the window through the reset action", async () => {
    view();
    expect(
      screen.getByRole("heading", { name: "Restore window" }),
    ).toBeVisible();
    expect(
      screen.getByRole("button", { name: "Reset window bounds" }),
    ).toHaveAccessibleDescription(
      "Restore the default size and center the window.",
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Reset window bounds" }),
    );
    await waitFor(() => expect(resetWindow).toHaveBeenCalledOnce());
    expect(resizeWindow).not.toHaveBeenCalled();
  });

  it("locks window actions while resizing and enables them again after completion", async () => {
    let finishResize: (() => void) | undefined;
    resizeWindow.mockReturnValueOnce(
      new Promise<void>((resolve) => {
        finishResize = resolve;
      }),
    );
    view();
    const window = within(screen.getByRole("region", { name: "Window" }));
    const small = window.getByRole("button", { name: "Small (960 × 560)" });
    fireEvent.click(small);
    await waitFor(() => expect(small).toBeDisabled());
    expect(
      window.getByRole("button", { name: "Default (1200 × 800)" }),
    ).toBeDisabled();
    expect(
      window.getByRole("button", { name: "Large (1200 × 960)" }),
    ).toBeDisabled();
    expect(
      window.getByRole("button", { name: "Reset window bounds" }),
    ).toBeDisabled();
    finishResize?.();
    await waitFor(() => expect(small).toBeEnabled());
    expect(
      window.getByRole("button", { name: "Reset window bounds" }),
    ).toBeEnabled();
  });

  it("shows window preview errors and allows retrying", async () => {
    resizeWindow.mockRejectedValueOnce(new Error("Cannot resize window"));
    view();
    fireEvent.click(screen.getByRole("button", { name: "Large (1200 × 960)" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Cannot resize window",
    );
    expect(
      screen.getByRole("button", { name: "Reset window bounds" }),
    ).toBeEnabled();
    fireEvent.click(screen.getByRole("button", { name: "Large (1200 × 960)" }));
    await waitFor(() => expect(resizeWindow).toHaveBeenCalledTimes(2));
    await waitFor(() =>
      expect(screen.queryByRole("alert")).not.toBeInTheDocument(),
    );
  });

  it("keeps update simulation enabled when Developer Tools is reopened", () => {
    const first = view();
    fireEvent.click(
      screen.getByRole("switch", { name: "Simulate available update" }),
    );
    first.unmount();
    view();
    expect(
      screen.getByRole("switch", { name: "Simulate available update" }),
    ).toBeChecked();
  });

  it("toggles update simulation", () => {
    const simulateListener = vi.fn();
    const clearListener = vi.fn();
    window.addEventListener(DEV_SIMULATE_UPDATE_EVENT, simulateListener);
    window.addEventListener(DEV_CLEAR_SIMULATED_UPDATE_EVENT, clearListener);

    view();
    const switchControl = screen.getByRole("switch", {
      name: "Simulate available update",
    });

    fireEvent.click(switchControl);
    expect(simulateListener).toHaveBeenCalledOnce();
    expect(switchControl).toBeChecked();

    fireEvent.click(switchControl);
    expect(clearListener).toHaveBeenCalledOnce();
    expect(switchControl).not.toBeChecked();

    window.removeEventListener(DEV_SIMULATE_UPDATE_EVENT, simulateListener);
    window.removeEventListener(DEV_CLEAR_SIMULATED_UPDATE_EVENT, clearListener);
  });

  it("keeps the storage loading switch enabled when Developer Tools is reopened", async () => {
    const listener = vi.fn();
    window.addEventListener(STORAGE_LOADING_PREVIEW_EVENT, listener);
    const client = new QueryClient();
    const first = view(vi.fn(), {}, client);
    fireEvent.click(
      screen.getByRole("switch", { name: "Simulate storage loading" }),
    );
    await waitFor(() =>
      expect(
        screen.getByRole("switch", { name: "Simulate storage loading" }),
      ).toBeChecked(),
    );
    expect(listener).toHaveBeenCalledOnce();
    first.unmount();
    view(vi.fn(), {}, client);
    expect(
      screen.getByRole("switch", { name: "Simulate storage loading" }),
    ).toBeChecked();
    fireEvent.click(
      screen.getByRole("switch", { name: "Simulate storage loading" }),
    );
    await waitFor(() =>
      expect(
        screen.getByRole("switch", { name: "Simulate storage loading" }),
      ).not.toBeChecked(),
    );
    expect(listener).toHaveBeenCalledOnce();
    window.removeEventListener(STORAGE_LOADING_PREVIEW_EVENT, listener);
  });

  it("does not expose the storage preview in production", () => {
    vi.stubEnv("DEV", false);
    try {
      view();
      expect(
        screen.queryByRole("switch", { name: "Simulate storage loading" }),
      ).not.toBeInTheDocument();
    } finally {
      vi.unstubAllEnvs();
    }
  });

  it("opens the real startup preview with a predefined isolated scenario", async () => {
    openPreview.mockResolvedValueOnce(undefined);
    view();
    await waitFor(() =>
      expect(
        screen.getByRole("combobox", { name: "Startup test scenario" }),
      ).toBeEnabled(),
    );
    expect(screen.queryByText("Scenario")).not.toBeInTheDocument();
    fireEvent.click(
      screen.getByRole("combobox", { name: "Startup test scenario" }),
    );
    fireEvent.click(
      await screen.findByRole("option", {
        name: "Migration with temporary fixtures",
      }),
    );
    expect(
      screen.getByRole("combobox", { name: "Startup test scenario" }),
    ).not.toHaveAttribute("aria-describedby");
    fireEvent.click(screen.getByRole("button", { name: "Open preview" }));
    await waitFor(() => expect(openPreview).toHaveBeenCalledWith("migration"));
    expect(restartPreview).not.toHaveBeenCalled();
  });

  it("requests a startup test restart and reports launch failures", async () => {
    restartPreview.mockRejectedValueOnce(new Error("could not restart"));
    view();
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Restart with preview" }),
      ).toBeEnabled(),
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Restart with preview" }),
    );
    await waitFor(() =>
      expect(restartPreview).toHaveBeenCalledWith("noUpdate"),
    );
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "could not restart",
    );
  });

  it("turns Open preview into a red Close preview action and locks the scenario until closed", async () => {
    view();
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Open preview" }),
      ).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole("button", { name: "Open preview" }));
    const close = await screen.findByRole("button", { name: "Close preview" });
    await waitFor(() => expect(close).toBeEnabled());
    expect(close).toHaveAttribute("data-variant", "outline");
    expect(close).toHaveClass("text-destructive");
    expect(
      screen.getByRole("combobox", { name: "Startup test scenario" }),
    ).toBeDisabled();
    expect(
      screen.getByRole("button", { name: "Restart with preview" }),
    ).toBeDisabled();
    expect(openPreview).toHaveBeenCalledOnce();
    fireEvent.click(close);
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Open preview" }),
      ).toBeEnabled(),
    );
    expect(closePreview).toHaveBeenCalledOnce();
    expect(
      screen.getByRole("combobox", { name: "Startup test scenario" }),
    ).toBeEnabled();
  });

  it("updates the action when the preview is closed outside Developer Tools", async () => {
    previewWindow.open = true;
    view();
    await screen.findByRole("button", { name: "Close preview" });
    act(() => {
      previewWindow.open = false;
      for (const handler of previewListeners) handler({ payload: false });
    });
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Open preview" }),
      ).toBeEnabled(),
    );
    expect(closePreview).not.toHaveBeenCalled();
  });

  it("keeps the active preview closable after a close error and allows retrying", async () => {
    previewWindow.open = true;
    closePreview.mockRejectedValueOnce(
      new Error("Could not close the preview window"),
    );
    view();
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Close preview" }),
      ).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole("button", { name: "Close preview" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Could not close the preview window",
    );
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Close preview" }),
      ).toBeEnabled(),
    );
    expect(
      screen.getByRole("combobox", { name: "Startup test scenario" }),
    ).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Close preview" }));
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Open preview" }),
      ).toBeEnabled(),
    );
    expect(closePreview).toHaveBeenCalledTimes(2);
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("does not overwrite a close event with an older state query", async () => {
    let finishQuery: ((open: boolean) => void) | undefined;
    getPreviewOpen.mockImplementationOnce(
      () =>
        new Promise<boolean>((resolve) => {
          finishQuery = resolve;
        }),
    );
    view();
    await waitFor(() => expect(getPreviewOpen).toHaveBeenCalledOnce());
    act(() => {
      for (const handler of previewListeners) handler({ payload: false });
    });
    await act(async () => finishQuery?.(true));
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Open preview" }),
      ).toBeEnabled(),
    );
    expect(
      screen.queryByRole("button", { name: "Close preview" }),
    ).not.toBeInTheDocument();
  });
});
