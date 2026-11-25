import { useMutation, useQueryClient } from "@tanstack/react-query";
import { RotateCcwIcon, UnlinkIcon } from "lucide-react";
import { type ReactNode, useState } from "react";
import { toast } from "sonner";
import {
  reinstallInstanceModpack,
  unlinkInstanceModpack,
} from "@/api/commands";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { SOURCE_LABELS } from "@/features/discover/lib/discover-search";
import type { ModpackOrigin } from "@/features/instances/lib/modpack-content";
import { friendlyError, messageOf } from "@/lib/errors";
import type { LinkedModpack } from "@/lib/generated/LinkedModpack";

import { modpackSectionStyles as styles } from "./modpack-section.styles";

const REINSTALL_HINT =
  "Reinstalling installs the modpack again and resets this instance's mods, resource packs and shader packs to the pack's own, removing any you added. Worlds stay.";
const REINSTALL_FILE_HINT =
  "A modpack imported from a file cannot be reinstalled, because its archive is not kept after install.";
const UNLINK_HINT =
  "Unlinking permanently disconnects this instance from its modpack. You can then change its game version, but it will no longer receive modpack updates. Its files stay as they are.";

export function ModpackSection({
  folder,
  origin,
  linked,
  preparing,
}: {
  folder: string;
  origin: ModpackOrigin;
  linked: LinkedModpack | null;
  preparing: boolean;
}) {
  const queryClient = useQueryClient();
  const unlink = useMutation({
    mutationFn: () => unlinkInstanceModpack(folder),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["instances"] });
      toast.success("Modpack unlinked.");
    },
    onError: (error: unknown) => toast.error(friendlyError(messageOf(error))),
  });
  const reinstall = useMutation({
    mutationFn: () => reinstallInstanceModpack(folder),
    onSuccess: async (job) => {
      await queryClient.invalidateQueries({ queryKey: ["instances"] });
      await queryClient.invalidateQueries({
        queryKey: ["instance-content", folder],
      });
      if (job.status === "completed") toast.success("Modpack reinstalled.");
      else if (job.status === "failed")
        toast.error(job.error ?? "Reinstall failed.");
    },
    onError: (error: unknown) => toast.error(friendlyError(messageOf(error))),
  });
  const fromFile = origin.type === "modpackFile";
  const source = fromFile
    ? `Imported from a ${SOURCE_LABELS[origin.source]} file`
    : `${SOURCE_LABELS[origin.source]} modpack`;
  const busy = preparing || unlink.isPending || reinstall.isPending;

  return (
    <div className={styles.root}>
      <div className={styles.summary}>
        <p className={styles.summaryName}>
          {linked ? `${linked.name} ${linked.versionName}` : "Linked modpack"}
        </p>
        <p className={styles.summaryDetail}>{source}</p>
      </div>
      <ConfirmedAction
        title="Reinstall modpack"
        hint={fromFile ? REINSTALL_FILE_HINT : REINSTALL_HINT}
        icon={<RotateCcwIcon data-icon="inline-start" aria-hidden />}
        action="Reinstall"
        question="Reinstall this modpack?"
        disabled={busy || fromFile}
        onConfirm={() => reinstall.mutate()}
      />
      <ConfirmedAction
        title="Unlink modpack"
        hint={UNLINK_HINT}
        icon={<UnlinkIcon data-icon="inline-start" aria-hidden />}
        action="Unlink"
        question="Unlink this modpack?"
        disabled={busy}
        onConfirm={() => unlink.mutate()}
      />
    </div>
  );
}

/** A described action that asks before it runs, since it cannot be undone. */
function ConfirmedAction({
  title,
  hint,
  icon,
  action,
  question,
  disabled,
  onConfirm,
}: {
  title: string;
  hint: string;
  icon: ReactNode;
  action: string;
  question: string;
  disabled: boolean;
  onConfirm: () => void;
}) {
  const [confirming, setConfirming] = useState(false);
  return (
    <div className={styles.group}>
      <div className={styles.groupText}>
        <h3 className={styles.label}>{title}</h3>
        <p className={styles.hint}>{hint}</p>
      </div>
      <Button
        type="button"
        variant="warning"
        className={styles.action}
        disabled={disabled}
        onClick={() => setConfirming(true)}
      >
        {icon}
        {action}
      </Button>
      <Dialog open={confirming} onOpenChange={setConfirming}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{question}</DialogTitle>
            <DialogDescription>{hint}</DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="outline">
                Cancel
              </Button>
            </DialogClose>
            <Button
              type="button"
              variant="warning"
              onClick={() => {
                setConfirming(false);
                onConfirm();
              }}
            >
              {action}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
