import {
  CheckIcon,
  ChevronDownIcon,
  CircleAlertIcon,
  InfoIcon,
  ListIcon,
  TriangleAlertIcon,
} from "lucide-react";
import { useState } from "react";

import { Button } from "@/components/ui/button";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  formatLogLevelSelection,
  type LogLevel,
  type LogLevelSelection,
  levelMatchesSelection,
  toggleLogLevel,
} from "@/features/instances/lib/log-level";
import {
  effectiveLogLevels,
  parseLogLine,
  partToneClass,
} from "@/features/instances/lib/log-line-parts";
import type { ConsoleLine } from "@/features/instances/model/console-line";
import { cn } from "@/lib/utils";

const LEVEL_OPTIONS = [
  {
    value: "info" as const,
    label: "Info",
    icon: InfoIcon,
    tone: "text-info",
  },
  {
    value: "warn" as const,
    label: "Warn",
    icon: TriangleAlertIcon,
    tone: "text-warning",
  },
  {
    value: "error" as const,
    label: "Error",
    icon: CircleAlertIcon,
    tone: "text-destructive",
  },
];

function selectionTone(selected: LogLevelSelection): string {
  if (selected.size === 0) {
    return "text-foreground";
  }
  if (selected.size === 1) {
    const only = [...selected][0];
    return (
      LEVEL_OPTIONS.find((option) => option.value === only)?.tone ??
      "text-foreground"
    );
  }
  return "text-foreground";
}

function selectionIcon(selected: LogLevelSelection) {
  if (selected.size === 1) {
    const only = [...selected][0];
    return (
      LEVEL_OPTIONS.find((option) => option.value === only)?.icon ?? ListIcon
    );
  }
  return ListIcon;
}

function ConsoleLineView({
  line,
  effectiveLevel,
}: {
  line: ConsoleLine;
  effectiveLevel: LogLevel;
}) {
  const parts = parseLogLine(line, effectiveLevel);
  return (
    <div>
      {parts.map((part) => (
        <span
          key={`${line.id}-${part.kind}-${part.text}`}
          className={partToneClass(part)}
        >
          {part.text}
        </span>
      ))}
    </div>
  );
}

export function ConsoleSection({ lines }: { lines: ConsoleLine[] }) {
  const [selected, setSelected] = useState<LogLevelSelection>(() => new Set());
  const levels = effectiveLogLevels(lines);
  const visible = lines.flatMap((line, index) => {
    const effectiveLevel = levels[index] ?? "info";
    if (!levelMatchesSelection(effectiveLevel, selected)) {
      return [];
    }
    return [{ line, effectiveLevel }];
  });
  const TriggerIcon = selectionIcon(selected);
  const label = formatLogLevelSelection(selected);
  const logSelect =
    "cursor-text select-text selection:bg-muted-foreground/35 selection:text-foreground";

  return (
    <section
      aria-label="Game log"
      className="mt-6 flex min-h-0 flex-1 flex-col gap-2"
    >
      <div className="flex shrink-0 items-center justify-between gap-3">
        <h3 className="text-xs font-medium text-muted-foreground">Console</h3>
        <Popover>
          <PopoverTrigger asChild>
            <Button
              type="button"
              variant="outline"
              size="sm"
              aria-label="Log level"
              className={cn(
                "w-40 justify-between font-semibold",
                selectionTone(selected),
              )}
            >
              <span className="flex min-w-0 items-center gap-1.5">
                <TriggerIcon aria-hidden data-icon="inline-start" />
                <span className="truncate">{label}</span>
              </span>
              <ChevronDownIcon
                aria-hidden
                className="size-4 shrink-0 text-muted-foreground"
              />
            </Button>
          </PopoverTrigger>
          <PopoverContent align="end" className="w-40 p-1">
            <p className="px-2 py-1.5 text-xs font-medium text-muted-foreground">
              Log Level
            </p>
            {LEVEL_OPTIONS.map((option) => {
              const Icon = option.icon;
              const checked = selected.has(option.value);
              return (
                <button
                  key={option.value}
                  type="button"
                  className={cn(
                    "flex w-full cursor-pointer items-center gap-1.5 rounded-md px-2 py-1.5 text-sm font-semibold outline-none hover:bg-accent",
                    option.tone,
                  )}
                  aria-pressed={checked}
                  onClick={() => {
                    setSelected(toggleLogLevel(selected, option.value));
                  }}
                >
                  <Icon aria-hidden className="size-4 shrink-0" />
                  <span className="flex-1 text-left">{option.label}</span>
                  {checked ? (
                    <CheckIcon aria-hidden className="size-4 shrink-0" />
                  ) : (
                    <span className="size-4 shrink-0" aria-hidden />
                  )}
                </button>
              );
            })}
          </PopoverContent>
        </Popover>
      </div>
      <ScrollArea className="min-h-0 flex-1 rounded-lg border border-input bg-black/25">
        {lines.length === 0 ? (
          <p
            className={cn(
              logSelect,
              "p-3 font-mono text-sm leading-relaxed text-muted-foreground",
            )}
          >
            The game log will appear here.
          </p>
        ) : visible.length === 0 ? (
          <p
            className={cn(
              logSelect,
              "p-3 font-mono text-sm leading-relaxed text-muted-foreground",
            )}
          >
            No lines match this level.
          </p>
        ) : (
          <pre
            className={cn(logSelect, "p-3 font-mono text-sm leading-relaxed")}
          >
            {visible.map(({ line, effectiveLevel }) => (
              <ConsoleLineView
                key={line.id}
                line={line}
                effectiveLevel={effectiveLevel}
              />
            ))}
          </pre>
        )}
      </ScrollArea>
    </section>
  );
}
