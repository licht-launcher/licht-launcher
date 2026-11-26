import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

export type CloseRequest = { preventDefault: () => void };

/** Sent by the backend to the settings window once the launcher window closed. */
const LAUNCHER_CLOSED_EVENT = "launcher-closed";

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

/**
 * Runs `handler` when the launcher window has closed, then closes this window,
 * whatever the handler does: the app is ending. Resolves to a function that
 * stops listening.
 */
export function onLauncherClosed(
  handler: () => Promise<void>,
): Promise<() => void> {
  return listen(LAUNCHER_CLOSED_EVENT, async () => {
    try {
      await handler();
    } finally {
      await getCurrentWindow().destroy();
    }
  });
}
