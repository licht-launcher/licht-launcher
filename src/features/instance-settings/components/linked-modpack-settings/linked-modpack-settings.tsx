import { useMutation, useQueryClient } from "@tanstack/react-query";
import { PackageIcon, RotateCcwIcon, UnlinkIcon } from "lucide-react";
import { type ReactNode, useState } from "react";
import { toast } from "sonner";
import {
  changeInstanceModpackVersion,
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
import { ChangeModpackVersionDialog } from "@/features/instance-settings/components/change-modpack-version-dialog";
import { instanceContentKey } from "@/features/instances/lib/content-query";
import type { ModpackOrigin } from "@/features/instances/lib/modpack-content";
import { friendlyError, messageOf } from "@/lib/errors";
import type { LinkedModpack } from "@/lib/generated/LinkedModpack";
import { SOURCE_LABELS } from "@/lib/modpack-sources";

import { linkedModpackSettingsStyles as styles } from "./linked-modpack-settings.styles";

const REINSTALL_HINT =
  "Reinstalling installs the modpack again and resets this instance's mods, resource packs and shader packs to the pack's own, removing any you added. Worlds stay.";
const REINSTALL_FILE_HINT =
  "A modpack imported from a file cannot be reinstalled, because its archive is not kept after install.";
const VERSION_HINT =
  "The linked modpack sets this instance's platform and game version. Install another version of it; content you added stays.";
const VERSION_FILE_HINT =
  "A modpack imported from a file has no other versions to install.";
const UNLINK_HINT =
  "Unlinking permanently disconnects this instance from its modpack. You can then change its game version, but it will no longer receive modpack updates. Its files stay as they are.";

/** The pack a linked instance came from, with the actions that change the link. */
export function LinkedModpackSettings({
  folder,
  origin,
  linked,
  preparing,
  busy,
}: {
  folder: string;
  origin: ModpackOrigin;
  linked: LinkedModpack | null;
  preparing: boolean;
  /** Another action on the instance is running. */
  busy: boolean;
}) {
  const queryClient = useQueryClient();
  const unlink = useMutation({
    mutationFn: () => unlinkInstanceModpack(folder),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["instances"] });
      await queryClient.invalidateQueries({
        queryKey: instanceContentKey(folder),
      });
      toast.success("Modpack unlinked.");
    },
    onError: (error: unknown) => toast.error(friendlyError(messageOf(error))),
  });
  const reinstall = useMutation({
    mutationFn: () => reinstallInstanceModpack(folder),
    onSuccess: async (job) => {
      await queryClient.invalidateQueries({ queryKey: ["instances"] });
      await queryClient.invalidateQueries({
        queryKey: instanceContentKey(folder),
      });
      if (job.status === "completed") toast.success("Modpack reinstalled.");
      else if (job.status === "failed")
        toast.error(job.error ?? "Reinstall failed.");
    },
    onError: (error: unknown) => toast.error(friendlyError(messageOf(error))),
  });
  const changeVersion = useMutation({
    mutationFn: (versionId: string) =>
      changeInstanceModpackVersion(folder, versionId),
    onSuccess: (job) => {
      if (job.status === "completed") toast.success("Modpack version changed.");
      else if (job.status === "failed")
        toast.error(job.error ?? "Changing the modpack version failed.");
    },
    onError: (error: unknown) => toast.error(friendlyError(messageOf(error))),
    // Also after a failure: the change waits to retry, or a cancel put the old version back.
    onSettled: async () => {
      await queryClient.invalidateQueries({ queryKey: ["instances"] });
      await queryClient.invalidateQueries({
        queryKey: instanceContentKey(folder),
      });
    },
  });
  const [versionOpen, setVersionOpen] = useState(false);
  const fromFile = origin.type === "modpackFile";
  const source = fromFile
    ? `Imported from a ${SOURCE_LABELS[origin.source]} file`
    : `${SOURCE_LABELS[origin.source]} modpack`;
  const disabled =
    busy ||
    preparing ||
    unlink.isPending ||
    reinstall.isPending ||
    changeVersion.isPending;

  return (
    <div className={styles.root}>
      <div className={styles.summary}>
        <p className={styles.summaryLabel}>Linked modpack</p>
        <p className={styles.summaryName}>
          {linked ? `${linked.name} ${linked.versionName}` : "Linked modpack"}
        </p>
        <p className={styles.summaryDetail}>{source}</p>
      </div>
      <div className={styles.group}>
        <div className={styles.groupText}>
          <h3 className={styles.label}>Modpack version</h3>
          <p className={styles.hint}>
            {fromFile ? VERSION_FILE_HINT : VERSION_HINT}
          </p>
        </div>
        <Button
          type="button"
          variant="warning"
          className={styles.action}
          disabled={disabled || fromFile}
          onClick={() => setVersionOpen(true)}
        >
          <PackageIcon data-icon="inline-start" aria-hidden />
          Change version
        </Button>
        {origin.type === "modpack" ? (
          <ChangeModpackVersionDialog
            open={versionOpen}
            origin={origin}
            busy={changeVersion.isPending}
            onOpenChange={setVersionOpen}
            onSave={(versionId) => changeVersion.mutate(versionId)}
          />
        ) : null}
      </div>
      <ConfirmedAction
        title="Reinstall modpack"
        hint={fromFile ? REINSTALL_FILE_HINT : REINSTALL_HINT}
        icon={<RotateCcwIcon data-icon="inline-start" aria-hidden />}
        action="Reinstall"
        question="Reinstall this modpack?"
        disabled={disabled || fromFile}
        onConfirm={() => reinstall.mutate()}
      />
      <ConfirmedAction
        title="Unlink modpack"
        hint={UNLINK_HINT}
        icon={<UnlinkIcon data-icon="inline-start" aria-hidden />}
        action="Unlink"
        question="Unlink this modpack?"
        disabled={disabled}
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
