import { describe, expect, it } from "vitest";

import {
  fromEntry,
  optionalSize,
  toInstanceSettings,
} from "@/features/instance-settings/model/draft";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

const entry: InstanceEntry = {
  folder: "Survival",
  name: "Survival",
  versionId: "1.20.1",
  minMemoryMb: 512,
  maxMemoryMb: 4096,
  jvmArguments: ["-XX:+UseG1GC"],
  fullscreen: true,
  width: 1280,
  height: 720,
  overrideWindow: true,
  overrideMemory: false,
  overrideJava: true,
  overrideJvmArguments: false,
  javaPath: "C:/jdk/bin/java.exe",
  lastOpenedAt: null,
};

describe("instance draft", () => {
  it("round-trips launch fields for save_instance", () => {
    const draft = fromEntry(entry);
    expect(draft.jvmArguments).toBe("-XX:+UseG1GC");
    expect(draft.width).toBe("1280");
    expect(toInstanceSettings(draft, 1280, 720)).toEqual({
      overrideWindow: true,
      overrideMemory: false,
      overrideJava: true,
      overrideJvmArguments: false,
      javaPath: "C:/jdk/bin/java.exe",
      maxMemoryMb: 4096,
      jvmArguments: ["-XX:+UseG1GC"],
      fullscreen: true,
      width: 1280,
      height: 720,
    });
  });

  it("rejects invalid window sizes", () => {
    expect(optionalSize("")).toBeNull();
    expect(optionalSize("0")).toBe("invalid");
    expect(optionalSize("854")).toBe(854);
  });
});
