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

export type InstanceAction =
  | { kind: "rename"; folder: string; name: string }
  | { kind: "duplicate"; folder: string; name: string }
  | { kind: "delete"; folder: string; name: string };

export function InstanceActionDialog({
  action,
  draftName,
  busy,
  onOpenChange,
  onDraftNameChange,
  onDuplicate,
  onRename,
  onDelete,
}: {
  action: InstanceAction | null;
  draftName: string;
  busy: boolean;
  onOpenChange: (open: boolean) => void;
  onDraftNameChange: (name: string) => void;
  onDuplicate: () => void;
  onRename: () => void;
  onDelete: () => void;
}) {
  return (
    <Dialog open={action !== null} onOpenChange={onOpenChange}>
      <DialogContent>
        {action?.kind === "duplicate" || action?.kind === "rename" ? (
          <>
            <DialogHeader>
              <DialogTitle>
                {action.kind === "rename" ? "Rename" : "Duplicate"}{" "}
                {action.name}
              </DialogTitle>
              <DialogDescription>
                {action.kind === "rename"
                  ? "Change the display name. The folder on disk stays the same."
                  : "Saves are copied into the new instance."}
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
                  (action.kind === "rename" && draftName.trim() === action.name)
                }
                onClick={action.kind === "rename" ? onRename : onDuplicate}
              >
                {action.kind === "rename"
                  ? "Rename instance"
                  : "Duplicate instance"}
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
