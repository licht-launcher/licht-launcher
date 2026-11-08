import {
  ArrowDownIcon,
  CheckIcon,
  ChevronDownIcon,
  CircleAlertIcon,
  InfoIcon,
  ListIcon,
  TriangleAlertIcon,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

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

/** Decorative flowers for the empty console. */
export const CONSOLE_EMPTY_LILY = "❀ ✿ ❁";

const BOTTOM_THRESHOLD_PX = 48;
const SCROLLBAR_REVEAL_MS = 2000;

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

function viewportOf(root: HTMLElement | null): HTMLElement | null {
  return root?.querySelector("[data-slot='scroll-area-viewport']") ?? null;
}

function isNearBottom(element: HTMLElement): boolean {
  return (
    element.scrollHeight - element.scrollTop - element.clientHeight <=
    BOTTOM_THRESHOLD_PX
  );
}

export function ConsoleLogLevelFilter({
  selected,
  onSelectedChange,
}: {
  selected: LogLevelSelection;
  onSelectedChange: (next: LogLevelSelection) => void;
}) {
  const TriggerIcon = selectionIcon(selected);
  const label = formatLogLevelSelection(selected);

  return (
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
                onSelectedChange(toggleLogLevel(selected, option.value));
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
  );
}

export function ConsoleSection({
  lines,
  selected,
}: {
  lines: ConsoleLine[];
  selected: LogLevelSelection;
}) {
  const visible = useMemo(() => {
    const levels = effectiveLogLevels(lines);
    return lines.flatMap((line, index) => {
      const effectiveLevel = levels[index] ?? "info";
      if (!levelMatchesSelection(effectiveLevel, selected)) {
        return [];
      }
      return [{ line, effectiveLevel }];
    });
  }, [lines, selected]);
  const logSelect =
    "cursor-text select-text selection:bg-muted-foreground/35 selection:text-foreground";

  const rootRef = useRef<HTMLDivElement>(null);
  const stickToBottomRef = useRef(true);
  const [stickToBottom, setStickToBottom] = useState(true);
  const [revealScrollbar, setRevealScrollbar] = useState(false);
  const lastLineId = lines.at(-1)?.id;

  useEffect(() => {
    if (lastLineId === undefined) {
      setRevealScrollbar(false);
      return;
    }
    setRevealScrollbar(true);
    const timer = window.setTimeout(() => {
      setRevealScrollbar(false);
    }, SCROLLBAR_REVEAL_MS);
    return () => window.clearTimeout(timer);
  }, [lastLineId]);

  useEffect(() => {
    const viewport = viewportOf(rootRef.current);
    if (viewport === null) {
      return;
    }
    const onScroll = () => {
      const nearBottom = isNearBottom(viewport);
      stickToBottomRef.current = nearBottom;
      setStickToBottom(nearBottom);
    };
    viewport.addEventListener("scroll", onScroll, { passive: true });
    return () => {
      viewport.removeEventListener("scroll", onScroll);
    };
  }, []);

  useEffect(() => {
    if (visible.length === 0 || !stickToBottomRef.current) {
      return;
    }
    const viewport = viewportOf(rootRef.current);
    if (viewport === null) {
      return;
    }
    // Only content changes pin the viewport; scroll events must not interrupt a smooth jump.
    viewport.scrollTop = viewport.scrollHeight;
  }, [visible]);

  function jumpToBottom() {
    const viewport = viewportOf(rootRef.current);
    if (viewport !== null) {
      const reduceMotion = window.matchMedia(
        "(prefers-reduced-motion: reduce)",
      ).matches;
      viewport.scrollTo({
        top: viewport.scrollHeight,
        behavior: reduceMotion ? "auto" : "smooth",
      });
    }
  }

  return (
    <section
      aria-label="Game log"
      className="relative flex min-h-0 flex-1 flex-col"
      ref={rootRef}
    >
      <ScrollArea
        type={revealScrollbar ? "always" : "hover"}
        className="min-h-0 flex-1 rounded-lg border border-border/60 bg-black/20"
      >
        {lines.length === 0 ? (
          <div
            className={cn(
              logSelect,
              "flex flex-col gap-3 p-3 font-mono text-sm leading-relaxed text-muted-foreground",
            )}
          >
            <p>The game log will appear here.</p>
            <p
              aria-hidden
              className="select-none font-sans text-lg leading-none text-muted-foreground/70"
            >
              {CONSOLE_EMPTY_LILY}
            </p>
          </div>
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
      {!stickToBottom && visible.length > 0 ? (
        <Button
          type="button"
          size="icon-sm"
          variant="secondary"
          className="absolute right-3 bottom-3 z-10 shadow-sm"
          aria-label="Scroll to bottom"
          onClick={jumpToBottom}
        >
          <ArrowDownIcon aria-hidden />
        </Button>
      ) : null}
    </section>
  );
}
