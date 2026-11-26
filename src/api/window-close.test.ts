import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  type CloseRequest,
  onLauncherClosed,
  onWindowCloseRequested,
} from "@/api/window-close";

const { currentWindow, listen, events } = vi.hoisted(() => ({
  currentWindow: vi.fn(),
  listen: vi.fn(),
  events: new Map<string, () => unknown>(),
}));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: currentWindow }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

const onCloseRequested = vi.fn();
const destroy = vi.fn();
const stop = vi.fn();

describe("window close", () => {
  beforeEach(() => {
    events.clear();
    onCloseRequested.mockReset().mockResolvedValue(stop);
    destroy.mockReset().mockResolvedValue(undefined);
    stop.mockReset();
    currentWindow.mockReset().mockReturnValue({ onCloseRequested, destroy });
    listen
      .mockReset()
      .mockImplementation(async (name: string, handler: () => unknown) => {
        events.set(name, handler);
        return stop;
      });
  });

  it("runs the handler before the native window closes", async () => {
    const handler = vi.fn<(request: CloseRequest) => Promise<void>>();
    expect(await onWindowCloseRequested(handler)).toBe(stop);
    expect(onCloseRequested).toHaveBeenCalledExactlyOnceWith(handler);
  });

  it("waits for nothing outside a native window", async () => {
    currentWindow.mockImplementation(() => {
      throw new Error("not a Tauri window");
    });
    const unsubscribe = await onWindowCloseRequested(vi.fn());
    expect(onCloseRequested).not.toHaveBeenCalled();
    expect(() => unsubscribe()).not.toThrow();
  });

  it("closes the window once the handler has run after the launcher closed", async () => {
    let finish: (() => void) | undefined;
    const handler = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          finish = resolve;
        }),
    );
    expect(await onLauncherClosed(handler)).toBe(stop);
    const closing = events.get("launcher-closed")?.();
    expect(handler).toHaveBeenCalledOnce();
    expect(destroy).not.toHaveBeenCalled();
    finish?.();
    await closing;
    expect(destroy).toHaveBeenCalledOnce();
  });

  it("closes the window even when the handler fails", async () => {
    const handler = vi.fn(async () => {
      throw new Error("save failed");
    });
    await onLauncherClosed(handler);
    await expect(events.get("launcher-closed")?.()).rejects.toThrow(
      "save failed",
    );
    expect(destroy).toHaveBeenCalledOnce();
  });
});
