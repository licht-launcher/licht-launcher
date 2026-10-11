import type { ConsoleLine } from "@/features/instances/model/console-line";

export const LOG_LEVELS = ["info", "warn", "error"] as const;

export type LogLevel = (typeof LOG_LEVELS)[number];

export type LogLevelSelection = ReadonlySet<LogLevel>;

const LEVEL_LABEL: Record<LogLevel, string> = {
  info: "Info",
  warn: "Warn",
  error: "Error",
};

/** Best-effort level from Mojang-style log lines and stderr. */
export function classifyLogLine(line: ConsoleLine): LogLevel {
  if (line.stream === "stderr") {
    return "error";
  }
  const text = line.line;
  if (/\/(?:ERROR|FATAL)]/i.test(text)) {
    return "error";
  }
  if (/\/WARN(?:ING)?]/i.test(text)) {
    return "warn";
  }
  if (/\/INFO]/i.test(text)) {
    return "info";
  }
  if (/\b(?:ERROR|FATAL)\b/.test(text)) {
    return "error";
  }
  if (/\bWARN(?:ING)?\b/.test(text)) {
    return "warn";
  }
  return "info";
}

/** Empty selection means show every level (All). */
export function levelMatchesSelection(
  level: LogLevel,
  selected: LogLevelSelection,
): boolean {
  if (selected.size === 0) {
    return true;
  }
  return selected.has(level);
}

export function toggleLogLevel(
  selected: LogLevelSelection,
  level: LogLevel,
): Set<LogLevel> {
  const next = new Set(selected);
  if (next.has(level)) {
    next.delete(level);
  } else {
    next.add(level);
  }
  return next;
}

export function formatLogLevelSelection(selected: LogLevelSelection): string {
  if (selected.size === 0) {
    return "All";
  }
  return LOG_LEVELS.filter((level) => selected.has(level))
    .map((level) => LEVEL_LABEL[level])
    .join(" + ");
}
