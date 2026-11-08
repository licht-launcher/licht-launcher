import { PlusIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import { DialogFooter } from "@/components/ui/dialog";

import { scratchActionsStyles } from "./scratch-actions.styles";

export function ScratchActions({
  creating,
  canCreate,
  onCancel,
  onCreate,
}: {
  creating: boolean;
  canCreate: boolean;
  onCancel: () => void;
  onCreate: () => void;
}) {
  return (
    <DialogFooter className={scratchActionsStyles.root}>
      <Button
        type="button"
        variant="outline"
        disabled={creating}
        onClick={onCancel}
      >
        Cancel
      </Button>
      <Button
        type="button"
        disabled={!canCreate || creating}
        onClick={onCreate}
      >
        <PlusIcon data-icon="inline-start" aria-hidden />
        {creating ? "Creating…" : "Create"}
      </Button>
    </DialogFooter>
  );
}
