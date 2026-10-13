import {
  BlocksIcon,
  BoxIcon,
  ClockIcon,
  type LucideIcon,
  PlayIcon,
  Settings2Icon,
  SquareIcon,
} from "lucide-react";
import type { ReactNode } from "react";

import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { InstanceIcon } from "@/features/instances/components/instance-icon";
import type { PlayPhase } from "@/features/instances/hooks/use-play-session";
import {
  barValue,
  isPreparingProgress,
  progressText,
} from "@/features/instances/lib/download-progress";
import { describeInstanceVersion } from "@/features/instances/lib/instance-version";
import { formatRelativeOpenedAt } from "@/features/instances/lib/relative-time";
import type { DownloadProgress } from "@/lib/generated/DownloadProgress";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";
import { cn } from "@/lib/utils";

function MetaTip({
  label,
  icon: Icon,
  children,
}: {
  label: string;
  icon: LucideIcon;
  children: ReactNode;
}) {
  return (
    <Tooltip>
      <TooltipTrigger
        type="button"
        className="inline-flex min-w-0 cursor-default items-center gap-1 rounded-sm text-sm text-muted-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring"
      >
        <Icon aria-hidden className="size-3.5 shrink-0" />
        {children}
      </TooltipTrigger>
      <TooltipContent side="bottom" align="center" sideOffset={2}>
        <Icon aria-hidden />
        {label}
      </TooltipContent>
    </Tooltip>
  );
}

export function PlaySection({
  instance,
  busy,
  playDisabled,
  phase,
  installing,
  progress,
  onPlay,
  onStop,
  onConfigure,
}: {
  instance: InstanceEntry | null;
  busy: boolean;
  playDisabled: boolean;
  phase: PlayPhase;
  installing: boolean;
  progress: DownloadProgress | null;
  onPlay: () => void;
  onStop: () => void;
  onConfigure: () => void;
}) {
  if (instance === null) {
    return (
      <p className="text-sm text-muted-foreground">
        Create an instance to get started.
      </p>
    );
  }

  const starting = phase === "starting";
  const running = phase === "running";
  const stopping = phase === "stopping";
  const showStop = running || stopping;
  const { gameVersion, loaderLabel } = describeInstanceVersion(
    instance.versionId,
  );
  const openedLabel =
    formatRelativeOpenedAt(instance.lastOpenedAt) ?? "Not opened yet";

  return (
    <div className="flex flex-wrap items-start justify-between gap-3">
      <div className="flex min-w-0 flex-1 flex-wrap items-start gap-4">
        <div className="flex min-w-0 items-center gap-3">
          <InstanceIcon size="md" />
          <div className="flex min-w-0 flex-col gap-0.5">
            <h2 className="text-lg font-medium">{instance.name}</h2>
            <TooltipProvider delayDuration={300}>
              <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-0.5">
                <MetaTip label="Game version" icon={BoxIcon}>
                  <span className="truncate">{gameVersion}</span>
                </MetaTip>
                <span aria-hidden className="text-sm text-muted-foreground">
                  •
                </span>
                <MetaTip label="Loader" icon={BlocksIcon}>
                  <span className="truncate">{loaderLabel}</span>
                </MetaTip>
                <span aria-hidden className="text-sm text-muted-foreground">
                  •
                </span>
                <MetaTip label="Last played" icon={ClockIcon}>
                  <span className="truncate">{openedLabel}</span>
                </MetaTip>
              </div>
            </TooltipProvider>
          </div>
        </div>
        {installing ? (
          <div className="flex min-w-0 max-w-sm flex-1 flex-col gap-1 pt-1">
            <span className="text-xs text-muted-foreground">
              {progress !== null && !isPreparingProgress(progress)
                ? progressText(progress)
                : "Preparing…"}
            </span>
            <Progress
              aria-label="Install progress"
              value={
                progress !== null && !isPreparingProgress(progress)
                  ? barValue(progress)
                  : 0
              }
            />
          </div>
        ) : null}
      </div>
      <div className="flex shrink-0 flex-wrap items-center gap-2">
        {showStop ? (
          <Button
            type="button"
            variant="destructive"
            disabled={busy || stopping}
            className={cn(stopping && "animate-pulse disabled:opacity-100")}
            onClick={onStop}
          >
            <SquareIcon data-icon="inline-start" aria-hidden />
            {stopping ? "Stopping…" : "Stop"}
          </Button>
        ) : (
          <Button
            disabled={busy || playDisabled || installing || starting}
            className={cn(starting && "animate-pulse disabled:opacity-100")}
            onClick={onPlay}
          >
            <PlayIcon data-icon="inline-start" aria-hidden />
            {starting ? "Starting…" : "Play"}
          </Button>
        )}
        <Button
          type="button"
          variant="outline"
          disabled={busy}
          aria-label={`Configure ${instance.name}`}
          onClick={onConfigure}
        >
          <Settings2Icon data-icon="inline-start" aria-hidden />
          Configure
        </Button>
      </div>
    </div>
  );
}
