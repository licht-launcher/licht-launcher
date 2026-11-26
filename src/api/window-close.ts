import { getCurrentWindow } from "@tauri-apps/api/window";

export type CloseRequest = { preventDefault: () => void };

/**
 * Runs `handler` before this window closes; the window waits for it and stays
 * open when it calls `preventDefault`. Resolves to a function that stops it.
 */
export async function onWindowCloseRequested(
  handler: (request: CloseRequest) => Promise<void>,
): Promise<() => void> {
  let window: ReturnType<typeof getCurrentWindow>;
  try {
    window = getCurrentWindow();
  } catch {
    // Not a native window (the web preview): there is no close to wait for.
    return () => {};
  }
  return window.onCloseRequested(handler);
}
