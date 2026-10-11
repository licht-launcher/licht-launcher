import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { createVersionChoices } from "@/features/instances/model/create-versions";
import type { VersionSummary } from "@/lib/generated/VersionSummary";

export type InstanceAction =
  | { kind: "create" }
  | { kind: "duplicate"; folder: string; name: string }
  | { kind: "delete"; folder: string; name: string };

export function InstanceActionDialog({
  action,
  draftName,
  draftVersion,
  includeSnapshots,
  versions,
  versionsPending,
  busy,
  onOpenChange,
  onDraftNameChange,
  onDraftVersionChange,
  onIncludeSnapshotsChange,
  onCreate,
  onDuplicate,
  onDelete,
}: {
  action: InstanceAction | null;
  draftName: string;
  draftVersion: string;
  includeSnapshots: boolean;
  versions: VersionSummary[];
  versionsPending: boolean;
  busy: boolean;
  onOpenChange: (open: boolean) => void;
  onDraftNameChange: (name: string) => void;
  onDraftVersionChange: (versionId: string) => void;
  onIncludeSnapshotsChange: (include: boolean) => void;
  onCreate: () => void;
  onDuplicate: () => void;
  onDelete: () => void;
}) {
  const versionChoices = createVersionChoices(versions, includeSnapshots);

  return (
    <Dialog open={action !== null} onOpenChange={onOpenChange}>
      <DialogContent>
        {action?.kind === "create" ? (
          <>
            <DialogHeader>
              <DialogTitle>Create instance</DialogTitle>
              <DialogDescription>
                Choose a name and a game version. Missing versions install when
                you play.
              </DialogDescription>
            </DialogHeader>
            <label
              className="flex flex-col gap-1 text-sm"
              htmlFor="instance-name"
            >
              Name
              <Input
                id="instance-name"
                value={draftName}
                onChange={(event) => {
                  onDraftNameChange(event.target.value);
                }}
              />
            </label>
            <div className="flex items-center justify-between gap-3">
              <div className="flex min-w-0 flex-col gap-1">
                <label htmlFor="include-snapshots" className="text-sm">
                  Include snapshots
                </label>
                <p
                  id="include-snapshots-hint"
                  className="text-sm text-muted-foreground"
                >
                  Show snapshot builds in the version list.
                </p>
              </div>
              <Switch
                id="include-snapshots"
                className="shrink-0"
                aria-label="Include snapshots"
                aria-describedby="include-snapshots-hint"
                checked={includeSnapshots}
                onCheckedChange={(next) => {
                  onIncludeSnapshotsChange(next);
                  const nextChoices = createVersionChoices(versions, next);
                  if (
                    !nextChoices.some((version) => version.id === draftVersion)
                  ) {
                    onDraftVersionChange(nextChoices[0]?.id ?? "");
                  }
                }}
              />
            </div>
            {versionChoices.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                {versionsPending
                  ? "Loading versions…"
                  : "No versions available."}
              </p>
            ) : (
              <Select value={draftVersion} onValueChange={onDraftVersionChange}>
                <SelectTrigger aria-label="Version">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {versionChoices.map((version) => (
                    <SelectItem key={version.id} value={version.id}>
                      {version.id}
                      {version.installed ? "" : " (will install)"}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}
            <DialogFooter>
              <Button
                disabled={
                  busy || draftName.trim() === "" || draftVersion === ""
                }
                onClick={onCreate}
              >
                Create instance
              </Button>
            </DialogFooter>
          </>
        ) : null}
        {action?.kind === "duplicate" ? (
          <>
            <DialogHeader>
              <DialogTitle>Duplicate {action.name}</DialogTitle>
              <DialogDescription>
                Saves are copied into the new instance.
              </DialogDescription>
            </DialogHeader>
            <label
              className="flex flex-col gap-1 text-sm"
              htmlFor="instance-name"
            >
              Name
              <Input
                id="instance-name"
                value={draftName}
                onChange={(event) => {
                  onDraftNameChange(event.target.value);
                }}
              />
            </label>
            <DialogFooter>
              <Button
                disabled={busy || draftName.trim() === ""}
                onClick={onDuplicate}
              >
                Duplicate instance
              </Button>
            </DialogFooter>
          </>
        ) : null}
        {action?.kind === "delete" ? (
          <>
            <DialogHeader>
              <DialogTitle>Delete {action.name}?</DialogTitle>
              <DialogDescription>
                Saves in this instance will be removed.
              </DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button variant="destructive" disabled={busy} onClick={onDelete}>
                Delete instance
              </Button>
            </DialogFooter>
          </>
        ) : null}
      </DialogContent>
    </Dialog>
  );
}
