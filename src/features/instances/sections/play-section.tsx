import { PlayIcon, Settings2Icon, SquareIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { InstanceIcon } from "@/features/instances/components/instance-icon";
import type { PlayPhase } from "@/features/instances/hooks/use-play-session";
import {
  barValue,
  progressText,
} from "@/features/instances/lib/download-progress";
import type { DownloadProgress } from "@/lib/generated/DownloadProgress";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";
import { cn } from "@/lib/utils";

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

  return (
    <div className="flex flex-wrap items-start justify-between gap-3">
      <div className="flex min-w-0 flex-1 flex-wrap items-start gap-4">
        <div className="flex min-w-0 items-center gap-3">
          <InstanceIcon size="md" />
          <div className="flex min-w-0 flex-col gap-1">
            <h2 className="text-lg font-medium">{instance.name}</h2>
            <p className="text-sm text-muted-foreground">
              {instance.versionId}
            </p>
          </div>
        </div>
        {installing ? (
          <div className="flex min-w-0 max-w-sm flex-1 flex-col gap-1 pt-1">
            <span className="text-xs text-muted-foreground">
              {progress ? progressText(progress) : "Preparing…"}
            </span>
            <Progress
              aria-label="Install progress"
              value={progress ? barValue(progress) : 0}
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
          variant="secondary"
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
