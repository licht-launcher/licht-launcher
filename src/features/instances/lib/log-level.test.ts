import { describe, expect, it } from "vitest";

import {
  classifyLogLine,
  formatLogLevelSelection,
  levelMatchesSelection,
  toggleLogLevel,
} from "@/features/instances/lib/log-level";
import type { ConsoleLine } from "@/features/instances/model/console-line";

function line(
  text: string,
  stream: ConsoleLine["stream"] = "stdout",
): ConsoleLine {
  return { id: 1, stream, line: text };
}

describe("classifyLogLine", () => {
  it("reads Mojang thread markers", () => {
    expect(classifyLogLine(line("[19:00:00] [Server thread/INFO]: Done"))).toBe(
      "info",
    );
    expect(
      classifyLogLine(line("[19:00:00] [Server thread/WARN]: Missing")),
    ).toBe("warn");
    expect(
      classifyLogLine(line("[19:00:00] [Server thread/ERROR]: Crash")),
    ).toBe("error");
    expect(
      classifyLogLine(line("[19:00:00] [Server thread/FATAL]: Dead")),
    ).toBe("error");
  });

  it("treats stderr as error", () => {
    expect(classifyLogLine(line("anything", "stderr"))).toBe("error");
  });
});

describe("levelMatchesSelection", () => {
  it("treats an empty selection as all levels", () => {
    const empty = new Set<"info" | "warn" | "error">();
    expect(levelMatchesSelection("info", empty)).toBe(true);
    expect(levelMatchesSelection("warn", empty)).toBe(true);
    expect(levelMatchesSelection("error", empty)).toBe(true);
  });

  it("matches any selected level", () => {
    const selected = new Set<"info" | "warn" | "error">(["warn", "error"]);
    expect(levelMatchesSelection("info", selected)).toBe(false);
    expect(levelMatchesSelection("warn", selected)).toBe(true);
    expect(levelMatchesSelection("error", selected)).toBe(true);
  });
});

describe("toggleLogLevel", () => {
  it("adds and removes levels", () => {
    const once = toggleLogLevel(new Set(), "warn");
    expect([...once]).toEqual(["warn"]);
    const twice = toggleLogLevel(once, "warn");
    expect([...twice]).toEqual([]);
    const both = toggleLogLevel(once, "error");
    expect([...both].sort()).toEqual(["error", "warn"]);
  });
});

describe("formatLogLevelSelection", () => {
  it("summarizes the selection for the trigger", () => {
    expect(formatLogLevelSelection(new Set())).toBe("All");
    expect(formatLogLevelSelection(new Set(["info"]))).toBe("Info");
    expect(formatLogLevelSelection(new Set(["error", "warn"]))).toBe(
      "Warn + Error",
    );
  });
});
