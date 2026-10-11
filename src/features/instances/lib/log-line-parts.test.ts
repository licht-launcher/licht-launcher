import { describe, expect, it } from "vitest";

import {
  effectiveLogLevels,
  parseLogLine,
} from "@/features/instances/lib/log-line-parts";
import type { ConsoleLine } from "@/features/instances/model/console-line";

function line(
  text: string,
  stream: ConsoleLine["stream"] = "stdout",
  id = 1,
): ConsoleLine {
  return { id, stream, line: text };
}

describe("parseLogLine", () => {
  it("splits a timed Mojang line", () => {
    const parts = parseLogLine(
      line("[19:00:00] [Server thread/INFO]: Done (3s)!"),
    );
    expect(parts).toEqual([
      { kind: "timestamp", text: "[19:00:00]" },
      { kind: "separator", text: " " },
      { kind: "thread", text: "[Server thread/" },
      { kind: "level", text: "INFO", level: "info" },
      { kind: "thread", text: "]" },
      { kind: "separator", text: ": " },
      { kind: "message", text: "Done (3s)!" },
    ]);
  });

  it("splits a short bracket line", () => {
    const parts = parseLogLine(line("[t/WARN]: careful"));
    expect(parts).toEqual([
      { kind: "thread", text: "[t/" },
      { kind: "level", text: "WARN", level: "warn" },
      { kind: "thread", text: "]" },
      { kind: "separator", text: ": " },
      { kind: "message", text: "careful" },
    ]);
  });

  it("keeps unmatched and stderr as raw", () => {
    expect(parseLogLine(line("plain text"))).toEqual([
      { kind: "raw", text: "plain text", level: "info" },
    ]);
    expect(parseLogLine(line("oops", "stderr"))).toEqual([
      { kind: "raw", text: "oops", level: "error" },
    ]);
  });
});

describe("effectiveLogLevels", () => {
  it("keeps stack traces at the previous error level", () => {
    const levels = effectiveLogLevels([
      line(
        "[20:11:09] [Render thread/ERROR]: Failed to verify authentication",
        "stdout",
        1,
      ),
      line(
        "com.mojang.authlib.exceptions.InvalidCredentialsException: Status: 401",
        "stdout",
        2,
      ),
      line(
        "\tat com.mojang.authlib.yggdrasil.YggdrasilUserAuthentication.authenticate(YggdrasilUserAuthentication.java:123)",
        "stdout",
        3,
      ),
      line("[20:11:09] [Render thread/INFO]: Setting user: Steve", "stdout", 4),
    ]);
    expect(levels).toEqual(["error", "error", "error", "info"]);
  });
});
