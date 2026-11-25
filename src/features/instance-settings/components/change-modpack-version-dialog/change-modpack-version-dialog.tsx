import { useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Spinner } from "@/components/ui/spinner";
import { useModpackVersions } from "@/features/discover/hooks/use-modpacks";
import type { StoreModpackOrigin } from "@/features/instances/lib/modpack-content";

import { changeModpackVersionDialogStyles as styles } from "./change-modpack-version-dialog.styles";

/** Picks another version of the store pack an instance is linked to. */
export function ChangeModpackVersionDialog({
  open,
  origin,
  busy,
  onOpenChange,
  onSave,
}: {
  open: boolean;
  origin: StoreModpackOrigin;
  busy: boolean;
  onOpenChange: (open: boolean) => void;
  onSave: (versionId: string) => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className={styles.content}>
        <DialogHeader>
          <DialogTitle>Change modpack version</DialogTitle>
          <DialogDescription>
            The current version's mods, resource packs and shader packs are
            replaced. Content you added and worlds stay.
          </DialogDescription>
        </DialogHeader>
        {/* Mounted only while open, so each opening starts from the installed version. */}
        <VersionForm
          origin={origin}
          busy={busy}
          onCancel={() => onOpenChange(false)}
          onSave={(versionId) => {
            onSave(versionId);
            onOpenChange(false);
          }}
        />
      </DialogContent>
    </Dialog>
  );
}

function VersionForm({
  origin,
  busy,
  onCancel,
  onSave,
}: {
  origin: StoreModpackOrigin;
  busy: boolean;
  onCancel: () => void;
  onSave: (versionId: string) => void;
}) {
  const versions = useModpackVersions(origin.source, origin.projectId);
  const [selected, setSelected] = useState(origin.versionId);

  return (
    <>
      <div className={styles.body}>
        <span className={styles.label} id="modpack-version-label">
          Version
        </span>
        {versions.isPending ? (
          <div className={styles.loading}>
            <Spinner className={styles.loadingIcon} aria-hidden />
            Loading versions…
          </div>
        ) : versions.isError ? (
          <div className={styles.errorBlock}>
            <p className={styles.message}>
              Could not load the modpack's versions. Check your connection and
              try again.
            </p>
            <Button
              type="button"
              variant="outline"
              className={styles.retryButton}
              onClick={() => {
                void versions.refetch();
              }}
            >
              Retry
            </Button>
          </div>
        ) : (
          <Select value={selected} disabled={busy} onValueChange={setSelected}>
            <SelectTrigger
              aria-labelledby="modpack-version-label"
              className={styles.selectTrigger}
            >
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {versions.data.map((version) => (
                <SelectItem key={version.id} value={version.id}>
                  {version.name}
                  {version.id === origin.versionId ? (
                    <span className={styles.installed}> (installed)</span>
                  ) : null}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        )}
      </div>
      <DialogFooter>
        <Button
          type="button"
          variant="outline"
          disabled={busy}
          onClick={onCancel}
        >
          Cancel
        </Button>
        <Button
          type="button"
          disabled={
            busy ||
            !versions.isSuccess ||
            selected === origin.versionId ||
            !versions.data.some((version) => version.id === selected)
          }
          onClick={() => onSave(selected)}
        >
          Install version
        </Button>
      </DialogFooter>
    </>
  );
}
