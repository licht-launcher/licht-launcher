import { useMutation, useQueryClient } from "@tanstack/react-query";
import { UnlinkIcon } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { unlinkInstanceModpack } from "@/api/commands";
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

import { linkedModpackSettingsStyles as styles } from "./linked-modpack-settings.styles";

const UNLINK_HINT =
  "Unlinking permanently disconnects this instance from its modpack. You can then change its game version, but it will no longer receive modpack updates. Its files stay as they are.";

/** The pack a linked instance came from, with the actions that change the link. */
export function LinkedModpackSettings({
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
  const [confirming, setConfirming] = useState(false);
  const unlink = useMutation({
    mutationFn: () => unlinkInstanceModpack(folder),
    onSuccess: async () => {
      setConfirming(false);
      await queryClient.invalidateQueries({ queryKey: ["instances"] });
      toast.success("Modpack unlinked.");
    },
    onError: (error: unknown) => toast.error(friendlyError(messageOf(error))),
  });
  const source =
    origin.type === "modpack"
      ? `${SOURCE_LABELS[origin.source]} modpack`
      : `Imported from a ${SOURCE_LABELS[origin.source]} file`;

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
          <h3 className={styles.label}>Unlink modpack</h3>
          <p className={styles.hint}>{UNLINK_HINT}</p>
        </div>
        <Button
          type="button"
          variant="warning"
          className={styles.action}
          disabled={preparing || unlink.isPending}
          onClick={() => setConfirming(true)}
        >
          <UnlinkIcon data-icon="inline-start" aria-hidden />
          Unlink
        </Button>
      </div>

      <Dialog open={confirming} onOpenChange={setConfirming}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Unlink this modpack?</DialogTitle>
            <DialogDescription>{UNLINK_HINT}</DialogDescription>
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
              disabled={unlink.isPending}
              onClick={() => unlink.mutate()}
            >
              Unlink
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
