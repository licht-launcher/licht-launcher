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
  | { kind: "duplicate"; folder: string; name: string }
  | { kind: "delete"; folder: string; name: string };

export function InstanceActionDialog({
  action,
  draftName,
  busy,
  onOpenChange,
  onDraftNameChange,
  onDuplicate,
  onDelete,
}: {
  action: InstanceAction | null;
  draftName: string;
  busy: boolean;
  onOpenChange: (open: boolean) => void;
  onDraftNameChange: (name: string) => void;
  onDuplicate: () => void;
  onDelete: () => void;
}) {
  return (
    <Dialog open={action !== null} onOpenChange={onOpenChange}>
      <DialogContent>
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
