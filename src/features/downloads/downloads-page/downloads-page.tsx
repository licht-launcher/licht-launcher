import {
  useIsMutating,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { Download, Pause, Play, Settings } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { EmptyState } from "@/components/ui/empty-state";
import { ScrollArea } from "@/components/ui/scroll-area";
import { SpeedChart } from "@/components/ui/speed-chart";
import { DownloadRow } from "@/features/downloads/components/downloads-popover";
import { SortableDownloadQueue } from "@/features/downloads/components/sortable-download-queue";
import { StoragePanel } from "@/features/downloads/components/storage-panel";
import {
  DOWNLOAD_JOBS_QUERY_KEY,
  useActiveDownloadId,
  useDownloadJobs,
} from "@/features/downloads/hooks/use-download-jobs";
import {
  DOWNLOAD_TELEMETRY_QUERY_KEY,
  useDownloadTelemetry,
} from "@/features/downloads/hooks/use-download-telemetry";
import {
  getSettings,
  listInstances,
  pauseAllDownloads,
  resumeAllDownloads,
} from "@/lib/commands";
import { formatDownloadLimit } from "@/lib/download-speed";
import { friendlyError, messageOf } from "@/lib/errors";
import { requestOpenSettings } from "@/lib/shell-events";

import { downloadsPageStyles } from "./downloads-page.styles";

export function DownloadsPage() {
  const jobs = useDownloadJobs();
  const current = jobs.data ?? [];
  const lastActiveId = useActiveDownloadId();
  const telemetry = useDownloadTelemetry();
  const instances = useQuery({
    queryKey: ["instances"],
    queryFn: listInstances,
  });
  const settings = useQuery({
    queryKey: ["settings"],
    queryFn: getSettings,
    retry: false,
  });
  const client = useQueryClient();
  const individualPending =
    useIsMutating({ mutationKey: ["download-action"] }) > 0;
  const action = useMutation({
    mutationKey: ["download-bulk-action"],
    mutationFn: (operation: "pause" | "resume") =>
      operation === "pause" ? pauseAllDownloads() : resumeAllDownloads(),
    onSuccess: async () => {
      await Promise.all([
        client.invalidateQueries({ queryKey: DOWNLOAD_JOBS_QUERY_KEY }),
        client.invalidateQueries({ queryKey: DOWNLOAD_TELEMETRY_QUERY_KEY }),
      ]);
    },
    onError: (error: unknown) => toast.error(friendlyError(messageOf(error))),
  });
  const running = current.filter((job) => job.status === "running");
  const pausedOwner =
    current.find((job) => job.id === lastActiveId && job.status === "paused") ??
    current.find((job) => job.status === "paused");
  const active =
    running.length > 0 ? running : pausedOwner ? [pausedOwner] : [];
  const activeIds = new Set(active.map((job) => job.id));
  const queued = current.filter((job) => !activeIds.has(job.id));
  const groups = [
    {
      kind: "active",
      label: "Active",
      entries: active,
    },
    { kind: "queued", label: "Up next", entries: queued },
  ];
  const reorderPending =
    useIsMutating({ mutationKey: ["download-reorder"] }) > 0;
  const busy = action.isPending || individualPending || reorderPending;
  const paused =
    telemetry.data?.paused === true ||
    (current.some((job) => job.status === "paused") &&
      !current.some(
        (job) => job.status === "running" || job.status === "queued",
      ));

  return (
    <section aria-label="Downloads" className={downloadsPageStyles.root}>
      <div className={downloadsPageStyles.header}>
        <h1 className={downloadsPageStyles.title}>
          <Download aria-hidden className={downloadsPageStyles.titleIcon} />
          Downloads
        </h1>
        <div className={downloadsPageStyles.actions}>
          <Button
            variant="secondary"
            size="sm"
            disabled={
              busy ||
              (!paused &&
                !current.some(
                  (job) => job.status === "running" || job.status === "queued",
                ))
            }
            onClick={() => action.mutate(paused ? "resume" : "pause")}
          >
            {paused ? <Play aria-hidden /> : <Pause aria-hidden />}
            {paused ? "Resume" : "Pause"}
          </Button>
          <Button
            variant="secondary"
            size="icon-sm"
            aria-label="Download settings"
            title="Download settings"
            onClick={() => requestOpenSettings("downloads")}
          >
            <Settings aria-hidden />
          </Button>
        </div>
      </div>
      <div className={downloadsPageStyles.layout}>
        <section aria-label="Queue" className={downloadsPageStyles.queue}>
          {jobs.isError && (
            <div role="alert" className={downloadsPageStyles.alert}>
              <span>Could not load downloads.</span>
              <Button
                variant="outline"
                size="sm"
                onClick={() => void jobs.refetch()}
              >
                Retry
              </Button>
            </div>
          )}
          {!jobs.isPending && !jobs.isError && current.length === 0 ? (
            <EmptyState
              icon={<Download />}
              title="No downloads in progress"
              description="Your next download will appear here."
            />
          ) : current.length > 0 ? (
            <ScrollArea className={downloadsPageStyles.scrollArea}>
              <div className={downloadsPageStyles.groups}>
                {groups
                  .filter((group) => group.entries.length > 0)
                  .map((group) => (
                    <section
                      key={group.kind}
                      aria-label={group.label}
                      className={downloadsPageStyles.group}
                    >
                      <h3 className={downloadsPageStyles.groupTitle}>
                        {group.kind === "queued"
                          ? `${group.label} (${group.entries.length})`
                          : group.label}
                      </h3>
                      {group.kind === "queued" ? (
                        <SortableDownloadQueue
                          jobs={group.entries}
                          instances={instances.data ?? []}
                          disabled={busy}
                        />
                      ) : (
                        <div className={downloadsPageStyles.list}>
                          {group.entries.map((job) => (
                            <DownloadRow
                              key={job.id}
                              job={job}
                              instance={instances.data?.find(
                                (instance) =>
                                  instance.folder === job.instanceFolder,
                              )}
                            />
                          ))}
                        </div>
                      )}
                    </section>
                  ))}
              </div>
            </ScrollArea>
          ) : null}
        </section>
        <aside
          aria-label="Download information"
          className={downloadsPageStyles.aside}
        >
          <ScrollArea className={downloadsPageStyles.asideScroll}>
            <div className={downloadsPageStyles.asideContent}>
              <SpeedChart
                title="Transfer speed"
                series={[
                  {
                    label: "Download",
                    samples: telemetry.samples.received,
                    color: "violet",
                    kind: "bars",
                  },
                  {
                    label: "Write",
                    samples: telemetry.samples.written,
                    color: "cool",
                  },
                ]}
              />
              {settings.data?.settings.downloadSpeedLimitBytesPerSecond !=
                null && (
                <p className={downloadsPageStyles.note}>
                  Limited to{" "}
                  {formatDownloadLimit(
                    settings.data.settings.downloadSpeedLimitBytesPerSecond,
                  )}
                </p>
              )}
              {settings.isError && (
                <p role="alert" className={downloadsPageStyles.error}>
                  Could not load download settings.
                </p>
              )}
              <StoragePanel active={running.length > 0} />
              {telemetry.data?.paused && (
                <p className={downloadsPageStyles.note}>
                  Downloads paused. An ongoing preparation step will finish
                  before pausing.
                </p>
              )}
              {telemetry.subscriptionError && (
                <p role="alert" className={downloadsPageStyles.error}>
                  Live speed updates are unavailable.
                </p>
              )}
              {telemetry.isError && (
                <p role="alert" className={downloadsPageStyles.error}>
                  Could not load download telemetry.
                </p>
              )}
              {instances.isError && (
                <p role="alert" className={downloadsPageStyles.error}>
                  Instance details are unavailable.
                </p>
              )}
            </div>
          </ScrollArea>
        </aside>
      </div>
    </section>
  );
}
