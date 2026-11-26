import { LogicalSize } from "@tauri-apps/api/dpi";
import { Window } from "@tauri-apps/api/window";

// Test sizes stay above the minimum size in src-tauri/src/lib.rs::create_launcher_window.
export const WINDOW_SIZE_PRESETS = [
  { id: "small", label: "Small", width: 960, height: 560 },
  { id: "default", label: "Default", width: 1200, height: 800 },
  { id: "large", label: "Large", width: 1200, height: 960 },
] as const;

export type WindowSizePreset = (typeof WINDOW_SIZE_PRESETS)[number]["id"];

// Developer Tools runs in the settings window; the presets are for the launcher.
async function launcherWindow(): Promise<Window> {
  const window = await Window.getByLabel("main");
  if (!window) throw new Error("The launcher window is not open");
  return window;
}

export async function previewWindowSize(id: WindowSizePreset): Promise<void> {
  const preset = WINDOW_SIZE_PRESETS.find((item) => item.id === id);
  if (!preset) throw new Error("Unsupported window size preset");
  const window = await launcherWindow();
  // A maximized window keeps its maximized state while resized.
  await window.unmaximize();
  await window.setSize(new LogicalSize(preset.width, preset.height));
}

export async function resetWindowBounds(): Promise<void> {
  await previewWindowSize("default");
  const window = await launcherWindow();
  await window.center();
}
