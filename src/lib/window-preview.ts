import { LogicalSize } from "@tauri-apps/api/dpi";
import { getCurrentWindow } from "@tauri-apps/api/window";

// Test sizes stay within the limits in src-tauri/src/lib.rs::create_main_window.
export const WINDOW_SIZE_PRESETS = [
  { id: "small", label: "Small", width: 960, height: 560 },
  { id: "default", label: "Default", width: 1200, height: 800 },
  { id: "large", label: "Large", width: 1200, height: 960 },
] as const;

export type WindowSizePreset = (typeof WINDOW_SIZE_PRESETS)[number]["id"];

export async function previewWindowSize(id: WindowSizePreset): Promise<void> {
  const preset = WINDOW_SIZE_PRESETS.find((item) => item.id === id);
  if (!preset) throw new Error("Unsupported window size preset");
  await getCurrentWindow().setSize(
    new LogicalSize(preset.width, preset.height),
  );
}

export async function resetWindowBounds(): Promise<void> {
  await previewWindowSize("default");
  await getCurrentWindow().center();
}
