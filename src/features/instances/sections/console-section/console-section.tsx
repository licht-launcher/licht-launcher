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

import { consoleSectionStyles } from "./console-section.styles";

/** Decorative flowers for the empty console. */
export const CONSOLE_EMPTY_LILY = "❀ ✿ ❁";

const BOTTOM_THRESHOLD_PX = 48;
const SCROLLBAR_REVEAL_MS = 2000;

const LEVEL_OPTIONS = [
  {
    value: "info" as const,
    label: "Info",
    icon: InfoIcon,
    tone: consoleSectionStyles.toneInfo,
  },
  {
    value: "warn" as const,
    label: "Warn",
    icon: TriangleAlertIcon,
    tone: consoleSectionStyles.toneWarn,
  },
  {
    value: "error" as const,
    label: "Error",
    icon: CircleAlertIcon,
    tone: consoleSectionStyles.toneError,
  },
];

function selectionTone(selected: LogLevelSelection): string {
  if (selected.size === 0) {
    return consoleSectionStyles.toneDefault;
  }
  if (selected.size === 1) {
    const only = [...selected][0];
    return (
      LEVEL_OPTIONS.find((option) => option.value === only)?.tone ??
      consoleSectionStyles.toneDefault
    );
  }
  return consoleSectionStyles.toneDefault;
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
            consoleSectionStyles.filterTrigger,
            selectionTone(selected),
          )}
        >
          <span className={consoleSectionStyles.filterLabel}>
            <TriggerIcon aria-hidden data-icon="inline-start" />
            <span className={consoleSectionStyles.filterLabelText}>
              {label}
            </span>
          </span>
          <ChevronDownIcon
            aria-hidden
            className={consoleSectionStyles.filterChevron}
          />
        </Button>
      </PopoverTrigger>
      <PopoverContent
        align="end"
        className={consoleSectionStyles.filterContent}
      >
        <p className={consoleSectionStyles.filterTitle}>Log Level</p>
        {LEVEL_OPTIONS.map((option) => {
          const Icon = option.icon;
          const checked = selected.has(option.value);
          return (
            <button
              key={option.value}
              type="button"
              className={cn(consoleSectionStyles.filterOption, option.tone)}
              aria-pressed={checked}
              onClick={() => {
                onSelectedChange(toggleLogLevel(selected, option.value));
              }}
            >
              <Icon
                aria-hidden
                className={consoleSectionStyles.filterOptionIcon}
              />
              <span className={consoleSectionStyles.filterOptionLabel}>
                {option.label}
              </span>
              {checked ? (
                <CheckIcon
                  aria-hidden
                  className={consoleSectionStyles.filterOptionIcon}
                />
              ) : (
                <span
                  className={consoleSectionStyles.filterOptionIcon}
                  aria-hidden
                />
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
      className={consoleSectionStyles.root}
      ref={rootRef}
    >
      <ScrollArea
        type={revealScrollbar ? "always" : "hover"}
        className={consoleSectionStyles.scrollArea}
      >
        {lines.length === 0 ? (
          <div
            className={cn(
              consoleSectionStyles.selectable,
              consoleSectionStyles.emptyLog,
            )}
          >
            <p>The game log will appear here.</p>
            <p aria-hidden className={consoleSectionStyles.emptyLogPrompt}>
              {CONSOLE_EMPTY_LILY}
            </p>
          </div>
        ) : visible.length === 0 ? (
          <p
            className={cn(
              consoleSectionStyles.selectable,
              consoleSectionStyles.logMessage,
            )}
          >
            No lines match this level.
          </p>
        ) : (
          <pre
            className={cn(
              consoleSectionStyles.selectable,
              consoleSectionStyles.log,
            )}
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
          className={consoleSectionStyles.jumpButton}
          aria-label="Scroll to bottom"
          onClick={jumpToBottom}
        >
          <ArrowDownIcon aria-hidden />
        </Button>
      ) : null}
    </section>
  );
}
