import { getCurrentWindow } from "@tauri-apps/api/window";
import { Download, Loader2, Minus, UserRound } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import {
  barValue,
  progressText,
} from "@/features/instances/lib/download-progress";
import { useInstallActivity } from "@/hooks/use-install-activity";
import { useTitleBarDecorations } from "@/hooks/use-title-bar-decorations";
import { usesNativeCaptionButtons } from "@/lib/native-caption";
import { requestOpenSettings } from "@/lib/shell-events";
import { cn } from "@/lib/utils";

const controlClass =
  "h-full w-14 shrink-0 rounded-none [&_svg:not([class*='size-'])]:size-4";

const chipClass =
  "h-6 max-w-36 gap-1.5 rounded-md border border-border/60 bg-muted/40 px-2 text-xs font-medium text-muted-foreground hover:bg-muted hover:text-foreground";

export function TitleBar() {
  const windowApi = getCurrentWindow();
  const { username } = useTitleBarDecorations();
  const { active, activeCount, progress, label } = useInstallActivity();
  const nativeCaption = usesNativeCaptionButtons();
  const downloadChipLabel = !active
    ? "Idle"
    : activeCount > 1
      ? String(activeCount)
      : progress !== null
        ? `${Math.floor(barValue(progress))}%`
        : "Idle";
  const downloadDetail =
    active && label !== null
      ? label
      : active && progress !== null
        ? progressText(progress)
        : "No downloads in progress";

  return (
    <header className="relative z-60 shrink-0 text-popover-foreground">
      <div
        className={cn(
          "flex h-9 items-stretch",
          nativeCaption && "pr-[var(--native-caption-inset)]",
        )}
      >
        <div data-tauri-drag-region className="min-w-0 flex-1" aria-hidden />
        <div className="flex shrink-0 items-center gap-1.5 px-2 text-muted-foreground">
          <Button
            type="button"
            variant="ghost"
            className={chipClass}
            aria-label={`Account: ${username}`}
            onClick={() => {
              requestOpenSettings("account");
            }}
          >
            <UserRound
              aria-hidden
              className="size-3.5 shrink-0"
              strokeWidth={2}
            />
            <span className="truncate">{username}</span>
          </Button>
          <Popover>
            <PopoverTrigger asChild>
              <Button
                type="button"
                variant="ghost"
                className={chipClass}
                aria-label={`Downloads: ${downloadChipLabel}`}
              >
                {active ? (
                  <Loader2
                    aria-hidden
                    className="size-3.5 shrink-0 animate-spin"
                    strokeWidth={2}
                  />
                ) : (
                  <Download
                    aria-hidden
                    className="size-3.5 shrink-0"
                    strokeWidth={2}
                  />
                )}
                <span className="tabular-nums">{downloadChipLabel}</span>
              </Button>
            </PopoverTrigger>
            <PopoverContent align="end" className="w-56 p-3">
              <p className="text-sm text-muted-foreground">{downloadDetail}</p>
            </PopoverContent>
          </Popover>
        </div>
        {nativeCaption ? null : (
          <div className="flex shrink-0 items-stretch text-muted-foreground">
            <Button
              type="button"
              variant="ghost"
              className={controlClass}
              aria-label="Minimize"
              onClick={() => {
                void windowApi.minimize();
              }}
            >
              <Minus aria-hidden className="size-4" strokeWidth={1.75} />
            </Button>
            <Button
              type="button"
              variant="ghost"
              className={cn(
                controlClass,
                "hover:bg-destructive/20 hover:text-destructive",
              )}
              aria-label="Close"
              onClick={() => {
                void windowApi.close();
              }}
            >
              <svg
                aria-hidden
                viewBox="0 0 24 24"
                className="size-5"
                fill="none"
              >
                <path
                  d="M7 7l10 10M17 7L7 17"
                  stroke="currentColor"
                  strokeWidth={1.75}
                  strokeLinecap="round"
                />
              </svg>
            </Button>
          </div>
        )}
      </div>
    </header>
  );
}
