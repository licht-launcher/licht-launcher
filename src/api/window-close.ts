import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

export type CloseRequest = { preventDefault: () => void };

const LAUNCHER_CLOSED_EVENT = "launcher-closed";

export async function onWindowCloseRequested(
  handler: (request: CloseRequest) => Promise<void>,
): Promise<() => void> {
  let window: ReturnType<typeof getCurrentWindow>;
  try {
    window = getCurrentWindow();
  } catch {
    // Not a native window (web preview).
    return () => {};
  }
  return window.onCloseRequested(handler);
}

/** Closes this window after `handler`, even if it fails: the app is ending. */
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
