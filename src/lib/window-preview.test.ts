import { LogicalSize } from "@tauri-apps/api/dpi";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  previewWindowSize,
  resetWindowBounds,
  type WindowSizePreset,
} from "@/lib/window-preview";

const { setSize, center } = vi.hoisted(() => ({
  setSize: vi.fn(),
  center: vi.fn(),
}));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ setSize, center }),
}));

describe("window preview", () => {
  beforeEach(() => {
    setSize.mockReset().mockResolvedValue(undefined);
    center.mockReset().mockResolvedValue(undefined);
  });

  it.each([
    ["small", 960, 560],
    ["default", 1200, 800],
    ["large", 1200, 960],
  ] as const)(
    "applies the %s preset in logical pixels",
    async (preset, width, height) => {
      await previewWindowSize(preset);
      expect(setSize).toHaveBeenCalledExactlyOnceWith(
        new LogicalSize(width, height),
      );
      expect(center).not.toHaveBeenCalled();
    },
  );

  it("restores the default size before centering the window", async () => {
    let finishResize: (() => void) | undefined;
    setSize.mockReturnValueOnce(
      new Promise<void>((resolve) => {
        finishResize = resolve;
      }),
    );
    const resetting = resetWindowBounds();
    expect(setSize).toHaveBeenCalledWith(new LogicalSize(1200, 800));
    expect(center).not.toHaveBeenCalled();
    finishResize?.();
    await resetting;
    expect(center).toHaveBeenCalledOnce();
  });

  it("propagates resize errors without centering a partially reset window", async () => {
    setSize.mockRejectedValueOnce(new Error("Cannot resize window"));
    await expect(resetWindowBounds()).rejects.toThrow("Cannot resize window");
    expect(center).not.toHaveBeenCalled();
  });

  it("reports a centering failure", async () => {
    center.mockRejectedValueOnce(new Error("Cannot center window"));
    await expect(resetWindowBounds()).rejects.toThrow("Cannot center window");
  });

  it("rejects an unsupported preset before invoking native APIs", async () => {
    await expect(
      previewWindowSize("unknown" as WindowSizePreset),
    ).rejects.toThrow("Unsupported window size preset");
    expect(setSize).not.toHaveBeenCalled();
    expect(center).not.toHaveBeenCalled();
  });
});
