import { useEffect, useState } from "react";

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
import type { VersionSummary } from "@/lib/generated/VersionSummary";

import { editInstanceVersionDialogStyles } from "./edit-instance-version-dialog.styles";

function releaseOptions(
  versions: VersionSummary[],
  versionId: string,
): VersionSummary[] {
  const releases = versions.filter(
    (version) => version.versionType === "release",
  );
  if (releases.some((version) => version.id === versionId)) {
    return releases;
  }
  const current = versions.find((version) => version.id === versionId);
  return current ? [current, ...releases] : releases;
}

export function EditInstanceVersionDialog({
  open,
  versionId,
  versions,
  versionsPending,
  versionsError,
  onRetryVersions,
  busy,
  onOpenChange,
  onSave,
}: {
  open: boolean;
  versionId: string;
  versions: VersionSummary[];
  versionsPending: boolean;
  versionsError: boolean;
  onRetryVersions: () => void;
  busy: boolean;
  onOpenChange: (open: boolean) => void;
  onSave: (versionId: string, installed: boolean) => void;
}) {
  const [draftVersion, setDraftVersion] = useState(versionId);
  const versionChoices = releaseOptions(versions, versionId);

  useEffect(() => {
    if (open) {
      setDraftVersion(versionId);
    }
  }, [open, versionId]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className={editInstanceVersionDialogStyles.content}>
        <DialogHeader>
          <DialogTitle>Edit game version</DialogTitle>
          <DialogDescription>
            Choose a Vanilla release. Missing versions are installed before the
            change is saved.
          </DialogDescription>
        </DialogHeader>
        <div className={editInstanceVersionDialogStyles.body}>
          <div className={editInstanceVersionDialogStyles.field}>
            <span className={editInstanceVersionDialogStyles.label}>
              Platform
            </span>
            <div className={editInstanceVersionDialogStyles.value}>Vanilla</div>
          </div>
          <div className={editInstanceVersionDialogStyles.field}>
            <span
              className={editInstanceVersionDialogStyles.label}
              id="edit-game-version-label"
            >
              Game version
            </span>
            {versionsPending ? (
              <div className={editInstanceVersionDialogStyles.loading}>
                <Spinner
                  className={editInstanceVersionDialogStyles.loadingIcon}
                  aria-hidden
                />
                Loading versions…
              </div>
            ) : versionsError && versionChoices.length === 0 ? (
              <div className={editInstanceVersionDialogStyles.errorBlock}>
                <p className={editInstanceVersionDialogStyles.message}>
                  Could not load the version list. Check your connection and try
                  again.
                </p>
                <Button
                  type="button"
                  variant="outline"
                  className={editInstanceVersionDialogStyles.retryButton}
                  onClick={onRetryVersions}
                >
                  Retry
                </Button>
              </div>
            ) : (
              <Select
                value={draftVersion}
                disabled={busy || versionChoices.length === 0}
                onValueChange={setDraftVersion}
              >
                <SelectTrigger
                  aria-labelledby="edit-game-version-label"
                  className={editInstanceVersionDialogStyles.selectTrigger}
                >
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {versionChoices.map((version) => (
                    <SelectItem key={version.id} value={version.id}>
                      {version.id}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}
          </div>
        </div>
        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            disabled={busy}
            onClick={() => {
              onOpenChange(false);
            }}
          >
            Cancel
          </Button>
          <Button
            type="button"
            disabled={
              busy ||
              versionsPending ||
              draftVersion === "" ||
              draftVersion === versionId
            }
            onClick={() => {
              const installed =
                versions.find((version) => version.id === draftVersion)
                  ?.installed ?? false;
              onSave(draftVersion, installed);
              onOpenChange(false);
            }}
          >
            Save
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
