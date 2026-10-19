import {
  useIsMutating,
  useMutation,
  useQueryClient,
} from "@tanstack/react-query";
import { Pause, Play, X } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import {
  DOWNLOAD_JOBS_QUERY_KEY,
  replaceDownloadSnapshot,
} from "@/features/downloads/hooks/use-download-jobs";
import { VERSIONS_QUERY_KEY } from "@/features/instances/lib/versions-query";
import {
  cancelDownloadJob,
  pauseDownloadJob,
  resumeDownloadJob,
} from "@/lib/commands";
import { friendlyError, messageOf } from "@/lib/errors";
import type { DownloadJobSnapshot } from "@/lib/generated/DownloadJobSnapshot";

export function DownloadControls({
  job,
  title,
  compact = false,
  disabled = false,
}: {
  job: DownloadJobSnapshot;
  title: string;
  compact?: boolean;
  disabled?: boolean;
}) {
  const queryClient = useQueryClient();
  const actionPending =
    useIsMutating({ mutationKey: ["download-action", job.id] }) > 0;
  const [confirming, setConfirming] = useState(false);
  const mutation = useMutation({
    mutationKey: ["download-action", job.id],
    mutationFn: (action: "pause" | "resume" | "cancel") => {
      const command = {
        pause: pauseDownloadJob,
        resume: resumeDownloadJob,
        cancel: cancelDownloadJob,
      }[action];
      return command(job.id);
    },
    onSuccess: async (snapshot) => {
      queryClient.setQueryData<DownloadJobSnapshot[]>(
        DOWNLOAD_JOBS_QUERY_KEY,
        (current) => replaceDownloadSnapshot(current, snapshot),
      );
      setConfirming(false);
      await queryClient.invalidateQueries({ queryKey: ["instances"] });
      await queryClient.invalidateQueries({ queryKey: VERSIONS_QUERY_KEY });
    },
    onError: (error: unknown) => toast.error(friendlyError(messageOf(error))),
  });
  const resumable =
    job.status === "paused" ||
    job.status === "interrupted" ||
    job.status === "failed";
  const pausable =
    job.status === "queued" ||
    (job.status === "running" && job.phase === "downloading");
  const pending = disabled || actionPending;
  const cancelling = job.status === "cancelling";
  const cancelLabel = cancelling ? "Retry cancellation" : "Cancel download";

  return (
    <Dialog
      open={confirming}
      onOpenChange={(open) => {
        if (!mutation.isPending) setConfirming(open);
      }}
    >
      {!cancelling && (
        <Button
          type="button"
          variant={resumable ? "warning" : compact ? "ghost" : "secondary"}
          size={compact ? "icon" : "default"}
          aria-label={`${resumable ? "Resume" : "Pause"} download for ${title}`}
          disabled={pending || (!resumable && !pausable)}
          onClick={() => mutation.mutate(resumable ? "resume" : "pause")}
        >
          {resumable ? <Play aria-hidden /> : <Pause aria-hidden />}
          {!compact && (resumable ? "Resume" : "Pause")}
        </Button>
      )}
      <DialogTrigger asChild>
        <Button
          type="button"
          variant={compact ? "ghost" : "destructive"}
          size={compact ? "icon" : "default"}
          aria-label={`${cancelLabel} for ${title}`}
          disabled={pending}
        >
          <X aria-hidden />
          {!compact && (cancelling ? "Retry cancellation" : "Cancel")}
        </Button>
      </DialogTrigger>
      <DialogContent showCloseButton={!mutation.isPending}>
        <DialogHeader>
          <DialogTitle>
            {cancelLabel} for {title}?
          </DialogTitle>
          <DialogDescription>
            {cancelling
              ? "Cancellation has not finished. Retry to remove the incomplete instance and its remaining files."
              : job.instanceFolder === null
                ? "The download will be cancelled. Completed shared files will be kept."
                : "A new instance that has not finished installing will be removed from your library, along with its files. Existing instances and their files will be kept."}
          </DialogDescription>
          {cancelling && job.error && (
            <p role="alert" className="text-sm text-destructive">
              {job.error}
            </p>
          )}
        </DialogHeader>
        <DialogFooter>
          <DialogClose asChild>
            <Button variant="outline" disabled={mutation.isPending}>
              {cancelling ? "Close" : "Keep downloading"}
            </Button>
          </DialogClose>
          <Button
            variant="destructive"
            disabled={mutation.isPending}
            onClick={() => mutation.mutate("cancel")}
          >
            {mutation.isPending ? "Cancelling…" : cancelLabel}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
