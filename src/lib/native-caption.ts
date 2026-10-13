/** Native caption buttons are 46 logical px wide; the plugin always paints three. */
export const NATIVE_CAPTION_WIDTH_PX = 46 * 3;

/**
 * When true, Windows draws min / (disabled) max / close above the WebView.
 * Keep HTML controls in tests and on Linux.
 */
export function usesNativeCaptionButtons(): boolean {
  if (typeof window === "undefined") {
    return false;
  }
  const tauri =
    "__TAURI_INTERNALS__" in window ||
    "__TAURI__" in window ||
    "__TAURI_IPC__" in window;
  if (!tauri) {
    return false;
  }
  return navigator.userAgent.includes("Windows");
}
