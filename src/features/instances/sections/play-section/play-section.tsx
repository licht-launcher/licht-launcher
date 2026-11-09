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
import { DownloadControls } from "@/features/downloads/components/download-controls";
import { downloadStatusText } from "@/features/downloads/lib/download-status";
import { InstanceIcon } from "@/features/instances/components/instance-icon";
import type { PlayPhase } from "@/features/instances/hooks/use-play-session";
import {
  barValue,
  isPreparingProgress,
  progressText,
} from "@/features/instances/lib/download-progress";
import { describeInstanceVersion } from "@/features/instances/lib/instance-version";
import { formatRelativeOpenedAt } from "@/features/instances/lib/relative-time";
import type { DownloadJobSnapshot } from "@/lib/generated/DownloadJobSnapshot";
import type { DownloadProgress } from "@/lib/generated/DownloadProgress";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";
import { cn } from "@/lib/utils";

import { playSectionStyles } from "./play-section.styles";

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
      <TooltipTrigger type="button" className={playSectionStyles.metaTrigger}>
        <Icon aria-hidden className={playSectionStyles.metaIcon} />
        {children}
      </TooltipTrigger>
      <TooltipContent side="bottom" align="center" sideOffset={2}>
        <Icon aria-hidden />
        {label}
      </TooltipContent>
    </Tooltip>
  );
}

function formatSpeed(bytesPerSecond: number): string {
  if (bytesPerSecond < 1024 * 1024) {
    return `${Math.max(0, Math.round(bytesPerSecond / 1024))} KB/s`;
  }
  return `${(bytesPerSecond / (1024 * 1024)).toFixed(1)} MB/s`;
}

export function PlaySection({
  instance,
  busy,
  playDisabled,
  phase,
  installing,
  downloadJob,
  progress,
  speedBytesPerSecond = 0,
  onPlay,
  onStop,
  onConfigure,
}: {
  instance: InstanceEntry | null;
  busy: boolean;
  playDisabled: boolean;
  phase: PlayPhase;
  installing: boolean;
  downloadJob?: DownloadJobSnapshot;
  progress: DownloadProgress | null;
  speedBytesPerSecond?: number;
  onPlay: () => void;
  onStop: () => void;
  onConfigure: () => void;
}) {
  if (instance === null) {
    return (
      <p className={playSectionStyles.muted}>
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

  let downloadDetail = "Preparing game files and modpack content…";
  if (downloadJob !== undefined) {
    downloadDetail = downloadStatusText(downloadJob);
    if (
      downloadJob.status === "running" &&
      downloadJob.phase === "downloading" &&
      speedBytesPerSecond > 0
    ) {
      downloadDetail += ` · ${formatSpeed(speedBytesPerSecond)}`;
    }
  } else if (speedBytesPerSecond > 0) {
    downloadDetail = `Downloading content · ${formatSpeed(speedBytesPerSecond)}`;
  }

  return (
    <div className={playSectionStyles.root}>
      <div className={playSectionStyles.header}>
        <div className={playSectionStyles.identity}>
          <InstanceIcon iconPath={instance.iconPath} size="md" />
          <div className={playSectionStyles.identityText}>
            <h2 className={playSectionStyles.title}>{instance.name}</h2>
            <TooltipProvider delayDuration={300}>
              <div className={playSectionStyles.meta}>
                <MetaTip label="Game version" icon={BoxIcon}>
                  <span className={playSectionStyles.metaValue}>
                    {gameVersion ?? "Preparing"}
                  </span>
                </MetaTip>
                <span aria-hidden className={playSectionStyles.muted}>
                  •
                </span>
                <MetaTip label="Loader" icon={BlocksIcon}>
                  <span className={playSectionStyles.metaValue}>
                    {loaderLabel}
                  </span>
                </MetaTip>
                <span aria-hidden className={playSectionStyles.muted}>
                  •
                </span>
                <MetaTip label="Last played" icon={ClockIcon}>
                  <span className={playSectionStyles.metaValue}>
                    {openedLabel}
                  </span>
                </MetaTip>
              </div>
            </TooltipProvider>
          </div>
        </div>
        <div className={playSectionStyles.actions}>
          {showStop ? (
            <Button
              type="button"
              variant="destructive"
              disabled={busy || stopping}
              className={cn(stopping && playSectionStyles.busyButton)}
              onClick={onStop}
            >
              <SquareIcon data-icon="inline-start" aria-hidden />
              {stopping ? "Stopping…" : "Stop"}
            </Button>
          ) : downloadJob !== undefined && !starting ? (
            <DownloadControls
              key={downloadJob.id}
              job={downloadJob}
              title={instance.name}
              disabled={busy}
            />
          ) : (
            <Button
              disabled={busy || playDisabled || installing || starting}
              className={cn(starting && playSectionStyles.busyButton)}
              onClick={onPlay}
            >
              <PlayIcon data-icon="inline-start" aria-hidden />
              {starting
                ? "Starting…"
                : instance.installation !== undefined &&
                    instance.installation.state !== "ready"
                  ? "Install"
                  : "Play"}
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
      {installing || downloadJob !== undefined ? (
        <div className={playSectionStyles.download}>
          <span className={playSectionStyles.downloadLabel}>
            <span className={playSectionStyles.downloadTitle}>
              {progress !== null && !isPreparingProgress(progress)
                ? progressText(progress, downloadJob?.phase)
                : "Preparing…"}
            </span>
            <span className={playSectionStyles.downloadDetail}>
              {downloadDetail}
            </span>
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
  );
}
