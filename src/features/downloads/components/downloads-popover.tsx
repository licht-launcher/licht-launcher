import { useQuery } from "@tanstack/react-query";
import { Download, Loader2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { Progress } from "@/components/ui/progress";
import { ScrollArea } from "@/components/ui/scroll-area";
import { DownloadControls } from "@/features/downloads/components/download-controls";
import { useDownloadJobs } from "@/features/downloads/hooks/use-download-jobs";
import { InstanceIcon } from "@/features/instances/components/instance-icon";
import {
  barValue,
  isPreparingProgress,
  progressText,
} from "@/features/instances/lib/download-progress";
import { listInstances } from "@/lib/commands";
import type { DownloadComponent } from "@/lib/generated/DownloadComponent";
import type { DownloadJobSnapshot } from "@/lib/generated/DownloadJobSnapshot";
import type { DownloadPhase } from "@/lib/generated/DownloadPhase";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

const components: Record<DownloadComponent, string> = {
  minecraft: "Minecraft",
  java: "Java",
  fabric: "Fabric",
  quilt: "Quilt",
  forge: "Forge",
  neoforge: "NeoForge",
  modpack: "Modpack",
};
const phases: Record<DownloadPhase, string> = {
  preparing: "Preparing",
  downloading: "Downloading",
  verifying: "Verifying",
  extracting: "Extracting",
  installing: "Installing",
};
const states = {
  queued: "Queued",
  paused: "Paused",
  interrupted: "Interrupted",
  failed: "Failed",
  completed: "Completed",
  cancelled: "Cancelled",
} as const;

function DownloadRow({
  job,
  instance,
}: {
  job: DownloadJobSnapshot;
  instance?: InstanceEntry;
}) {
  const title =
    job.javaMajor !== undefined
      ? `Java ${job.javaMajor}`
      : (instance?.name ?? job.instanceFolder ?? "Download");
  const known = !isPreparingProgress(job.progress);
  const detail = `${job.status === "running" ? phases[job.phase] : states[job.status]}${job.component ? ` · ${components[job.component]}` : ""}`;
  return (
    <section
      aria-label={`Download for ${title}`}
      className="flex flex-col gap-2 rounded-lg border border-border p-3"
    >
      <div className="flex items-center gap-2">
        {job.javaMajor !== undefined ? (
          <Download
            aria-hidden
            className="size-6 shrink-0 text-muted-foreground"
          />
        ) : (
          <InstanceIcon iconPath={instance?.iconPath} />
        )}
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm font-medium">{title}</p>
          <p className="text-xs text-muted-foreground">{detail}</p>
        </div>
        <DownloadControls job={job} title={title} compact />
      </div>
      <Progress
        aria-label={`Progress for ${title}`}
        value={known ? barValue(job.progress) : null}
      />
      {known && (
        <p className="text-xs tabular-nums text-muted-foreground">
          {progressText(job.progress)}
        </p>
      )}
      {job.error && (
        <p role="alert" className="text-xs text-destructive">
          {job.error}
        </p>
      )}
    </section>
  );
}

export function DownloadsPopover({
  triggerClassName,
}: {
  triggerClassName?: string;
}) {
  const query = useDownloadJobs();
  const instances = useQuery({
    queryKey: ["instances"],
    queryFn: listInstances,
  });
  const jobs = query.data ?? [];
  const active = jobs.some(
    (job) => job.status === "running" || job.status === "queued",
  );
  const single = jobs.length === 1 ? jobs[0] : undefined;
  const label = query.isPending
    ? "Loading"
    : jobs.length === 0
      ? "Idle"
      : single === undefined
        ? String(jobs.length)
        : single.status === "running" && !isPreparingProgress(single.progress)
          ? `${Math.floor(barValue(single.progress))}%`
          : single.status === "running"
            ? phases[single.phase]
            : states[single.status];
  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          className={triggerClassName}
          aria-label={`Downloads: ${label}`}
        >
          {active ? (
            <Loader2 aria-hidden className="size-3.5 shrink-0 animate-spin" />
          ) : (
            <Download aria-hidden className="size-3.5 shrink-0" />
          )}
          <span className="tabular-nums">{label}</span>
        </Button>
      </PopoverTrigger>
      <PopoverContent align="end" className="w-96 p-3">
        <div className="flex flex-col gap-3">
          <p className="text-sm font-medium">Downloads</p>
          {query.isPending && (
            <p className="text-sm text-muted-foreground">Loading downloads…</p>
          )}
          {query.isError && (
            <div
              role="alert"
              className="flex items-center justify-between gap-2 text-sm"
            >
              <span>Could not load downloads.</span>
              <Button
                variant="outline"
                size="sm"
                onClick={() => void query.refetch()}
              >
                Retry
              </Button>
            </div>
          )}
          {!query.isPending && !query.isError && jobs.length === 0 && (
            <p className="text-sm text-muted-foreground">
              No downloads in progress
            </p>
          )}
          {instances.isError && (
            <p role="alert" className="text-xs text-destructive">
              Instance details are unavailable.
            </p>
          )}
          {jobs.length > 0 && (
            <ScrollArea className={jobs.length > 2 ? "h-80" : "h-auto"}>
              <div className="flex flex-col gap-2">
                {jobs.map((job) => (
                  <DownloadRow
                    key={job.id}
                    job={job}
                    instance={instances.data?.find(
                      (instance) => instance.folder === job.instanceFolder,
                    )}
                  />
                ))}
              </div>
            </ScrollArea>
          )}
        </div>
      </PopoverContent>
    </Popover>
  );
}
