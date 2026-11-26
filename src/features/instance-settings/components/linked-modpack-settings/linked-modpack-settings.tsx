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
import { Card } from "@/components/ui/card";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { formatUpdatedAt } from "@/features/discover/lib/format";
import { ChangeModpackVersionDialog } from "@/features/instance-settings/components/change-modpack-version-dialog";
import { InstanceIcon } from "@/features/instances/components/instance-icon";
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
  iconPath,
  preparing,
  busy,
}: {
  folder: string;
  origin: ModpackOrigin;
  linked: LinkedModpack | null;
  /** The pack icon cached for the instance. */
  iconPath: string | null;
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
  const released = formatUpdatedAt(linked?.publishedAt ?? null);
  const details = [
    linked?.author ? `by ${linked.author}` : null,
    source,
    released ? `Released ${released}` : null,
  ].filter((detail) => detail !== null);
  const disabled =
    busy ||
    preparing ||
    unlink.isPending ||
    reinstall.isPending ||
    changeVersion.isPending;

  return (
    <>
      <Card title="Linked modpack">
        <div className={styles.pack}>
          <InstanceIcon iconPath={iconPath} size="md" />
          <div className={styles.packText}>
            <p className={styles.packName}>
              {linked?.name ?? "Linked modpack"}
            </p>
            {linked ? (
              <p className={styles.packVersion}>{linked.versionName}</p>
            ) : null}
            <p className={styles.packDetails}>{details.join(" · ")}</p>
          </div>
          <Button
            type="button"
            variant="warning"
            className={styles.control}
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
        <p className={styles.rowDescription}>
          {fromFile ? VERSION_FILE_HINT : VERSION_HINT}
        </p>
      </Card>
      <Card title="Manage modpack">
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
      </Card>
    </>
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
    <div className={styles.row}>
      <div className={styles.rowText}>
        <p className={styles.rowLabel}>{title}</p>
        <p className={styles.rowDescription}>{hint}</p>
      </div>
      <Button
        type="button"
        variant="warning"
        className={styles.control}
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
