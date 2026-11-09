import { useQuery } from "@tanstack/react-query";
import { Download, Loader2 } from "lucide-react";
import type { ReactNode } from "react";
import { listInstances } from "@/api/commands";
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
import {
  downloadStatusLabel,
  downloadStatusText,
} from "@/features/downloads/lib/download-status";
import { InstanceIcon } from "@/features/instances/components/instance-icon";
import {
  barValue,
  isPreparingProgress,
  progressText,
} from "@/features/instances/lib/download-progress";
import type { DownloadJobSnapshot } from "@/lib/generated/DownloadJobSnapshot";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";
import { cn } from "@/lib/utils";

import { downloadsPopoverStyles } from "./downloads-popover.styles";

export function DownloadRow({
  job,
  instance,
  dragHandle,
}: {
  job: DownloadJobSnapshot;
  instance?: InstanceEntry;
  dragHandle?: ReactNode;
}) {
  const title =
    job.javaMajor !== undefined
      ? `Java ${job.javaMajor}`
      : (instance?.name ?? job.instanceFolder ?? "Download");
  const known = !isPreparingProgress(job.progress);
  const completed = job.status === "completed";
  const modpack =
    job.component === "modpack" ||
    job.component === "modpackarchive" ||
    instance?.installation?.source.type === "modpack";
  const detail = downloadStatusText(job);
  return (
    <section
      aria-label={`Download for ${title}`}
      className={downloadsPopoverStyles.job}
    >
      <div className={downloadsPopoverStyles.jobHeader}>
        {dragHandle}
        {job.javaMajor !== undefined ? (
          <Download aria-hidden className={downloadsPopoverStyles.jobIcon} />
        ) : (
          <InstanceIcon
            iconPath={instance?.iconPath}
            size={modpack ? "md" : "sm"}
          />
        )}
        <div className={downloadsPopoverStyles.jobText}>
          <p className={downloadsPopoverStyles.jobTitle}>{title}</p>
          <p className={downloadsPopoverStyles.jobDetail}>{detail}</p>
        </div>
        {!completed && <DownloadControls job={job} title={title} compact />}
      </div>
      {!completed && (
        <Progress
          aria-label={`Progress for ${title}`}
          value={known ? barValue(job.progress) : null}
        />
      )}
      {known && !completed && (
        <p className={downloadsPopoverStyles.jobProgress}>
          {progressText(job.progress, job.phase)}
        </p>
      )}
      {job.error && (
        <p role="alert" className={downloadsPopoverStyles.jobError}>
          {job.error}
        </p>
      )}
    </section>
  );
}

export function DownloadsPopover({
  triggerClassName,
  side,
  showLabel = false,
  onOpen,
}: {
  triggerClassName?: string;
  side?: "top" | "bottom";
  showLabel?: boolean;
  onOpen?: () => void;
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
  const current =
    jobs.find(
      (job) => job.instanceFolder !== null && job.status === "running",
    ) ??
    jobs.find((job) => job.status === "running") ??
    jobs.find((job) => job.status === "queued") ??
    jobs[0];
  const currentInstance = instances.data?.find(
    (instance) => instance.folder === current?.instanceFolder,
  );
  const queued = jobs.filter(
    (job) => job.status === "queued" && job.id !== current?.id,
  ).length;
  const currentProgress =
    current !== undefined &&
    current.status !== "queued" &&
    !isPreparingProgress(current.progress)
      ? barValue(current.progress)
      : null;
  const label = query.isPending
    ? "Loading"
    : jobs.length === 0
      ? "Idle"
      : single === undefined
        ? String(jobs.length)
        : single.status === "running" && !isPreparingProgress(single.progress)
          ? `${Math.floor(barValue(single.progress))}%`
          : downloadStatusLabel(single);
  const trigger = (
    <Button
      type="button"
      variant="ghost"
      className={cn(
        triggerClassName,
        showLabel && current !== undefined && downloadsPopoverStyles.trigger,
      )}
      aria-label={`Downloads: ${label}`}
      onClick={onOpen}
    >
      {showLabel && current !== undefined ? (
        <>
          {current.javaMajor !== undefined ? (
            <Download
              aria-hidden
              className={downloadsPopoverStyles.triggerIcon}
            />
          ) : (
            <InstanceIcon iconPath={currentInstance?.iconPath} />
          )}
          <span className={downloadsPopoverStyles.triggerBody}>
            <span className={downloadsPopoverStyles.triggerHeader}>
              <span className={downloadsPopoverStyles.triggerTitle}>
                {downloadStatusText(current)}
              </span>
              <span className={downloadsPopoverStyles.triggerMeta}>
                {queued > 0 ? <span>{queued} queued</span> : null}
                {currentProgress !== null ? (
                  <span>{Math.floor(currentProgress)}%</span>
                ) : null}
              </span>
            </span>
            <Progress
              aria-label="Current download progress"
              value={currentProgress}
              className={downloadsPopoverStyles.triggerProgress}
            />
          </span>
        </>
      ) : (
        <>
          {active ? (
            <Loader2
              aria-hidden
              className={downloadsPopoverStyles.triggerSpinner}
            />
          ) : (
            <Download
              aria-hidden
              className={downloadsPopoverStyles.triggerDownloadIcon}
            />
          )}
          {showLabel ? <span>Downloads</span> : null}
          {showLabel && (label === "Idle" || query.isPending) ? null : (
            <span className={downloadsPopoverStyles.triggerCount}>{label}</span>
          )}
        </>
      )}
    </Button>
  );
  if (onOpen !== undefined) return trigger;
  return (
    <Popover>
      <PopoverTrigger asChild>{trigger}</PopoverTrigger>
      <PopoverContent
        side={side}
        align="end"
        className={downloadsPopoverStyles.content}
      >
        <div className={downloadsPopoverStyles.body}>
          <p className={downloadsPopoverStyles.title}>Downloads</p>
          {query.isError && (
            <div role="alert" className={downloadsPopoverStyles.toolbar}>
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
            <p className={downloadsPopoverStyles.empty}>
              No downloads in progress
            </p>
          )}
          {instances.isError && (
            <p role="alert" className={downloadsPopoverStyles.jobError}>
              Instance details are unavailable.
            </p>
          )}
          {jobs.length > 0 && (
            <ScrollArea
              className={
                jobs.length > 2
                  ? downloadsPopoverStyles.scrollAreaTall
                  : downloadsPopoverStyles.scrollAreaAuto
              }
            >
              <div className={downloadsPopoverStyles.jobs}>
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
