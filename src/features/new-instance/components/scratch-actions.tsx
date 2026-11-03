import { PlusIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import { DialogFooter } from "@/components/ui/dialog";

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
    <DialogFooter className="-mx-4 -mb-6 bg-transparent border-none px-4 pt-0 sm:justify-between">
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
