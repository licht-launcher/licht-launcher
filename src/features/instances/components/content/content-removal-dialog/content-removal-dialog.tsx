import { AlertTriangleIcon } from "lucide-react";
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
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";

export function ContentRemovalDialog({
  entry,
  pending,
  onOpenChange,
  onConfirm,
  onCloseAutoFocus,
}: {
  entry: InstanceContentEntry | null;
  pending: boolean;
  onOpenChange: (open: boolean) => void;
  onConfirm: (entry: InstanceContentEntry) => void;
  onCloseAutoFocus: (event: Event) => void;
}) {
  return (
    <Dialog open={entry !== null} onOpenChange={onOpenChange}>
      <DialogContent onCloseAutoFocus={onCloseAutoFocus}>
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <AlertTriangleIcon className="size-5 text-destructive" />
            Remove content?
          </DialogTitle>
          <DialogDescription>
            This will permanently delete <strong>{entry?.name}</strong> from
            this instance. This action cannot be undone by the launcher.
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <DialogClose asChild>
            <Button type="button" variant="outline">
              Cancel
            </Button>
          </DialogClose>
          <Button
            type="button"
            variant="destructive"
            disabled={pending || entry === null}
            onClick={() => {
              if (entry !== null) onConfirm(entry);
            }}
          >
            Remove content
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
