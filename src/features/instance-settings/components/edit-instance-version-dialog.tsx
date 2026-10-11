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
import type { VersionSummary } from "@/lib/generated/VersionSummary";

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
  busy,
  onOpenChange,
  onSave,
}: {
  open: boolean;
  versionId: string;
  versions: VersionSummary[];
  busy: boolean;
  onOpenChange: (open: boolean) => void;
  onSave: (versionId: string) => void;
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
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Edit game version</DialogTitle>
          <DialogDescription>
            Choose a Vanilla release. Missing versions are installed before the
            change is saved.
          </DialogDescription>
        </DialogHeader>
        <div className="flex flex-col gap-4">
          <div className="flex flex-col gap-1">
            <span className="text-sm font-medium">Platform</span>
            <div className="flex h-8 items-center rounded-lg bg-muted/40 px-2.5 text-sm">
              Vanilla
            </div>
          </div>
          <div className="flex flex-col gap-1">
            <span className="text-sm font-medium" id="edit-game-version-label">
              Game version
            </span>
            <Select
              value={draftVersion}
              disabled={busy || versionChoices.length === 0}
              onValueChange={setDraftVersion}
            >
              <SelectTrigger
                aria-labelledby="edit-game-version-label"
                className="w-full font-normal"
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
            disabled={busy || draftVersion === "" || draftVersion === versionId}
            onClick={() => {
              onSave(draftVersion);
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
