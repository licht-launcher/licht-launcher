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

import { instanceActionDialogStyles } from "./instance-action-dialog.styles";

export type InstanceAction =
  | { kind: "rename"; folder: string; name: string }
  | { kind: "delete"; folder: string; name: string };

export function InstanceActionDialog({
  action,
  draftName,
  busy,
  onOpenChange,
  onDraftNameChange,
  onRename,
  onDelete,
}: {
  action: InstanceAction | null;
  draftName: string;
  busy: boolean;
  onOpenChange: (open: boolean) => void;
  onDraftNameChange: (name: string) => void;
  onRename: () => void;
  onDelete: () => void;
}) {
  return (
    <Dialog open={action !== null} onOpenChange={onOpenChange}>
      <DialogContent>
        {action?.kind === "rename" ? (
          <>
            <DialogHeader>
              <DialogTitle>Rename {action.name}</DialogTitle>
              <DialogDescription>
                Change the display name. The folder on disk stays the same.
              </DialogDescription>
            </DialogHeader>
            <label
              className={instanceActionDialogStyles.field}
              htmlFor="instance-name"
            >
              Name
              <Input
                id="instance-name"
                value={draftName}
                disabled={busy}
                onChange={(event) => {
                  onDraftNameChange(event.target.value);
                }}
              />
            </label>
            <DialogFooter>
              <Button
                disabled={
                  busy ||
                  draftName.trim() === "" ||
                  draftName.trim() === action.name
                }
                onClick={onRename}
              >
                Rename instance
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
