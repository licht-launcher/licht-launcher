import { DragDropProvider } from "@dnd-kit/react";
import { isSortableOperation, useSortable } from "@dnd-kit/react/sortable";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { GripVertical } from "lucide-react";
import { useRef } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { DownloadRow } from "@/features/downloads/components/downloads-popover";
import { DOWNLOAD_JOBS_QUERY_KEY } from "@/features/downloads/hooks/use-download-jobs";
import { reorderDownloadJobs } from "@/lib/commands";
import { friendlyError, messageOf } from "@/lib/errors";
import type { DownloadJobSnapshot } from "@/lib/generated/DownloadJobSnapshot";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

function orderedSnapshots(
  current: DownloadJobSnapshot[],
  ids: string[],
): DownloadJobSnapshot[] {
  const selected = new Set(ids);
  const waiting = current.filter(
    (job) =>
      selected.has(job.id) &&
      (job.status === "queued" || job.status === "paused"),
  );
  const byId = new Map(waiting.map((job) => [job.id, job]));
  if (
    waiting.length !== ids.length ||
    new Set(ids).size !== ids.length ||
    ids.some((id) => !byId.has(id))
  )
    return current;
  let position = 0;
  return current.map((job) =>
    selected.has(job.id) ? (byId.get(ids[position++] ?? "") ?? job) : job,
  );
}

function SortableRow({
  job,
  instance,
  index,
  disabled,
}: {
  job: DownloadJobSnapshot;
  instance?: InstanceEntry;
  index: number;
  disabled: boolean;
}) {
  const { ref, handleRef, isDragSource } = useSortable({
    id: job.id,
    index,
    group: "waiting-downloads",
    disabled,
    transition: { duration: 150 },
  });
  const title = instance?.name ?? job.instanceFolder ?? "Download";
  return (
    <div ref={ref} className={isDragSource ? "opacity-60" : undefined}>
      <DownloadRow
        job={job}
        instance={instance}
        dragHandle={
          <Button
            ref={handleRef}
            type="button"
            variant="ghost"
            size="icon-sm"
            className="shrink-0 cursor-grab touch-none text-muted-foreground active:cursor-grabbing"
            aria-label={`Reorder download for ${title}`}
            disabled={disabled}
          >
            <GripVertical aria-hidden />
          </Button>
        }
      />
    </div>
  );
}

export function SortableDownloadQueue({
  jobs,
  instances,
  disabled = false,
}: {
  jobs: DownloadJobSnapshot[];
  instances: InstanceEntry[];
  disabled?: boolean;
}) {
  const client = useQueryClient();
  const started = useRef<string[] | null>(null);
  const waiting = jobs.filter(
    (job) => job.status === "queued" || job.status === "paused",
  );
  const reorder = useMutation({
    mutationKey: ["download-reorder"],
    mutationFn: reorderDownloadJobs,
    onMutate: async (ids) => {
      await client.cancelQueries({ queryKey: DOWNLOAD_JOBS_QUERY_KEY });
      const previous = (
        client.getQueryData<DownloadJobSnapshot[]>(DOWNLOAD_JOBS_QUERY_KEY) ??
        []
      )
        .filter((job) => ids.includes(job.id))
        .map((job) => job.id);
      client.setQueryData<DownloadJobSnapshot[]>(
        DOWNLOAD_JOBS_QUERY_KEY,
        (current = []) => orderedSnapshots(current, ids),
      );
      return previous;
    },
    onError: (error: unknown, _ids, previous) => {
      if (previous)
        client.setQueryData<DownloadJobSnapshot[]>(
          DOWNLOAD_JOBS_QUERY_KEY,
          (current = []) => orderedSnapshots(current, previous),
        );
      toast.error(friendlyError(messageOf(error)));
    },
    onSettled: () =>
      client.invalidateQueries({ queryKey: DOWNLOAD_JOBS_QUERY_KEY }),
  });

  return (
    <DragDropProvider
      onDragStart={() => {
        started.current = waiting.map((job) => job.id);
      }}
      onDragEnd={(event) => {
        const initial = started.current;
        started.current = null;
        if (
          event.canceled ||
          !event.operation.target ||
          !isSortableOperation(event.operation) ||
          !initial
        )
          return;
        const { source } = event.operation;
        if (!source) return;
        if (source.initialIndex === source.index) return;
        if (
          initial.length !== waiting.length ||
          initial.some((id, index) => id !== waiting[index]?.id) ||
          initial[source.initialIndex] !== source.id ||
          source.index < 0 ||
          source.index >= initial.length
        ) {
          toast.error("Download queue changed; try moving the download again.");
          return;
        }
        const ids = [...initial];
        const [moved] = ids.splice(source.initialIndex, 1);
        if (moved === undefined) return;
        ids.splice(source.index, 0, moved);
        reorder.mutate(ids);
      }}
    >
      <div className="divide-y divide-border/60">
        {jobs.map((job) =>
          job.status === "queued" || job.status === "paused" ? (
            <SortableRow
              key={job.id}
              job={job}
              index={waiting.findIndex(
                (waitingJob) => waitingJob.id === job.id,
              )}
              instance={instances.find(
                (instance) => instance.folder === job.instanceFolder,
              )}
              disabled={disabled || reorder.isPending}
            />
          ) : (
            <DownloadRow
              key={job.id}
              job={job}
              instance={instances.find(
                (instance) => instance.folder === job.instanceFolder,
              )}
            />
          ),
        )}
      </div>
    </DragDropProvider>
  );
}
