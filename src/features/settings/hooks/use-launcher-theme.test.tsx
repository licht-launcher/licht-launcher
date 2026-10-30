import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { toast } from "sonner";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { snapshot } from "@/features/settings/fixtures/snapshot";
import { useLauncherTheme } from "@/features/settings/hooks/use-launcher-theme";
import { getSettings } from "@/lib/commands";

const { listeners } = vi.hoisted(() => ({
  listeners: new Set<(event: { payload: unknown }) => void>(),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(
    async (_event: string, listener: (event: { payload: unknown }) => void) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  ),
}));
vi.mock("@/lib/commands", () => ({ getSettings: vi.fn() }));
vi.mock("sonner", () => ({ toast: { error: vi.fn() } }));

class ColorScheme extends EventTarget {
  matches = false;
}
let scheme: ColorScheme;

function renderTheme(startup = true) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );
  return renderHook(() => useLauncherTheme(startup), { wrapper });
}

describe("useLauncherTheme", () => {
  beforeEach(() => {
    scheme = new ColorScheme();
    vi.stubGlobal("matchMedia", () => scheme);
    vi.stubGlobal("__LICHT_THEME__", "dark");
    vi.clearAllMocks();
    listeners.clear();
    vi.mocked(getSettings).mockResolvedValue(snapshot);
  });
  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
    document.documentElement.classList.remove("dark");
  });

  it("applies the startup preference without querying services before startup is ready", () => {
    vi.stubGlobal("__LICHT_THEME__", "light");
    const { result } = renderTheme();
    expect(result.current).toBe("light");
    expect(document.documentElement).not.toHaveClass("dark");
    expect(getSettings).not.toHaveBeenCalled();
  });

  it("loads the persisted preference in the main window", async () => {
    vi.mocked(getSettings).mockResolvedValue({
      ...snapshot,
      settings: { ...snapshot.settings, theme: "light" },
    });
    const { result } = renderTheme(false);
    await waitFor(() => expect(result.current).toBe("light"));
    expect(document.documentElement).not.toHaveClass("dark");
  });

  it("follows OS changes only while system is selected", () => {
    vi.stubGlobal("__LICHT_THEME__", "system");
    const { result } = renderTheme();
    expect(result.current).toBe("light");
    act(() => {
      scheme.matches = true;
      scheme.dispatchEvent(new Event("change"));
    });
    expect(result.current).toBe("dark");
    act(() => {
      for (const listener of listeners) listener({ payload: "light" });
    });
    expect(result.current).toBe("light");
    act(() => {
      scheme.dispatchEvent(new Event("change"));
    });
    expect(result.current).toBe("light");
  });

  it("receives saved changes from other windows and rejects malformed events", () => {
    const { result } = renderTheme();
    act(() => {
      for (const listener of listeners) listener({ payload: "light" });
    });
    expect(result.current).toBe("light");
    act(() => {
      for (const listener of listeners) listener({ payload: "invalid" });
    });
    expect(result.current).toBe("light");
    expect(toast.error).toHaveBeenCalledWith(
      "Invalid launcher theme notification.",
    );
  });

  it("stops applying OS changes after unmounting", () => {
    vi.stubGlobal("__LICHT_THEME__", "system");
    const { unmount } = renderTheme();
    unmount();
    act(() => {
      scheme.matches = true;
      scheme.dispatchEvent(new Event("change"));
    });
    expect(document.documentElement).not.toHaveClass("dark");
  });
});
