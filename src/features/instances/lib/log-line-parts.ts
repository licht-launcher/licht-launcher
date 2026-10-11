import {
  classifyLogLine,
  type LogLevel,
} from "@/features/instances/lib/log-level";
import type { ConsoleLine } from "@/features/instances/model/console-line";

export type LogLinePart =
  | { kind: "timestamp"; text: string }
  | { kind: "thread"; text: string }
  | { kind: "level"; text: string; level: LogLevel }
  | { kind: "separator"; text: string }
  | { kind: "message"; text: string }
  | { kind: "raw"; text: string; level: LogLevel };

function mapLevel(tag: string): LogLevel {
  const upper = tag.toUpperCase();
  if (upper === "ERROR" || upper === "FATAL") {
    return "error";
  }
  if (upper === "WARN" || upper === "WARNING") {
    return "warn";
  }
  return "info";
}

const WITH_TIME =
  /^(\[\d{1,2}:\d{2}(?::\d{2})?\])\s+(\[([^\]/]+)\/(INFO|WARN(?:ING)?|ERROR|FATAL)\])(:\s*)(.*)$/i;

const WITHOUT_TIME =
  /^(\[([^\]/]+)\/(INFO|WARN(?:ING)?|ERROR|FATAL)\])(:\s*)(.*)$/i;

/** Exception lines, stack frames, and similar continuations after a leveled line. */
export function isStackContinuation(text: string): boolean {
  const trimmed = text.trimStart();
  return (
    /^at\s+/.test(trimmed) ||
    /^Caused by:/.test(trimmed) ||
    /^Suppressed:/.test(trimmed) ||
    /^\.\.\.\s+\d+\s+more/.test(trimmed) ||
    /^[a-zA-Z_$][\w.$]*(?:Exception|Error)(?::|\s|$)/.test(trimmed)
  );
}

/**
 * Levels for display/filter: stack traces inherit the previous structured level
 * so ERROR dumps stay red instead of falling back to info green.
 */
export function effectiveLogLevels(lines: ConsoleLine[]): LogLevel[] {
  let carry: LogLevel = "info";
  return lines.map((line) => {
    if (line.stream === "stderr") {
      carry = "error";
      return "error";
    }
    if (WITH_TIME.test(line.line) || WITHOUT_TIME.test(line.line)) {
      const level = classifyLogLine(line);
      carry = level;
      return level;
    }
    if (isStackContinuation(line.line)) {
      return carry;
    }
    const level = classifyLogLine(line);
    carry = level;
    return level;
  });
}

/** Split a Mojang-style log line into colored parts when the shape matches. */
export function parseLogLine(
  line: ConsoleLine,
  rawLevel?: LogLevel,
): LogLinePart[] {
  const text = line.line;

  const timed = WITH_TIME.exec(text);
  if (timed) {
    const [, timestamp, , thread, levelTag, separator, message] = timed;
    return [
      { kind: "timestamp", text: timestamp ?? "" },
      { kind: "separator", text: " " },
      { kind: "thread", text: `[${thread}/` },
      {
        kind: "level",
        text: levelTag ?? "",
        level: mapLevel(levelTag ?? "INFO"),
      },
      { kind: "thread", text: "]" },
      { kind: "separator", text: separator ?? ": " },
      { kind: "message", text: message ?? "" },
    ];
  }

  const plain = WITHOUT_TIME.exec(text);
  if (plain) {
    const [, , thread, levelTag, separator, message] = plain;
    return [
      { kind: "thread", text: `[${thread}/` },
      {
        kind: "level",
        text: levelTag ?? "",
        level: mapLevel(levelTag ?? "INFO"),
      },
      { kind: "thread", text: "]" },
      { kind: "separator", text: separator ?? ": " },
      { kind: "message", text: message ?? "" },
    ];
  }

  return [
    {
      kind: "raw",
      text,
      level: rawLevel ?? classifyLogLine(line),
    },
  ];
}

export function levelToneClass(level: LogLevel): string {
  if (level === "error") {
    return "text-destructive";
  }
  if (level === "warn") {
    return "text-warning";
  }
  return "text-info";
}

export function partToneClass(part: LogLinePart): string {
  switch (part.kind) {
    case "timestamp":
      return "text-muted-foreground";
    case "thread":
      return "text-muted-foreground/80";
    case "level":
      return levelToneClass(part.level);
    case "separator":
      return "text-muted-foreground";
    case "message":
      return "text-foreground";
    case "raw":
      return levelToneClass(part.level);
  }
}
