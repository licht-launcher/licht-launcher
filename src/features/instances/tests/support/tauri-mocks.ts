import { vi } from "vitest";

const { invoke, listeners, closeWindow } = vi.hoisted(() => ({
  invoke: vi.fn(),
  closeWindow: vi.fn(),
  listeners: new Map<string, (event: { payload: unknown }) => void>(),
}));

export { closeWindow, invoke, listeners };

vi.mock("@tauri-apps/api/core", () => ({
  invoke,
  convertFileSrc: (path: string) => `asset://localhost/${path}`,
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
    minimize: vi.fn(),
    close: closeWindow,
    setDecorations: vi.fn(),
  }),
}));
