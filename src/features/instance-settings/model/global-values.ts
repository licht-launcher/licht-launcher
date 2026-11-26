import type { Settings } from "@/lib/generated/Settings";
import { formatMemoryPreset } from "@/lib/memory-presets";

// Minecraft's own window size, used when global settings leave it empty.
const GAME_WIDTH = 854;
const GAME_HEIGHT = 480;

/** What an instance uses for each category while it does not override it. */
export function globalValues(settings: Settings) {
  const args = settings.jvmArguments.join(" ").trim();
  return {
    window: settings.fullscreen
      ? "Using global settings: full screen."
      : `Using global settings: ${settings.width ?? GAME_WIDTH} × ${settings.height ?? GAME_HEIGHT}.`,
    memory: `Using global settings: ${formatMemoryPreset(settings.maxMemoryMb)}.`,
    java: "Using the Java installation from global settings.",
    arguments:
      args === ""
        ? "Using global settings: no extra arguments."
        : `Using global settings: ${args}`,
  };
}
