import { ArrowLeftIcon, PlusIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import { DialogFooter } from "@/components/ui/dialog";

export function ScratchActions({
  busy,
  creating,
  canCreate,
  onBack,
  onCreate,
}: {
  busy: boolean;
  creating: boolean;
  canCreate: boolean;
  onBack: () => void;
  onCreate: () => void;
}) {
  return (
    <DialogFooter className="-mx-6 -mb-6 px-6 py-4 sm:justify-between">
      <Button type="button" variant="outline" disabled={busy} onClick={onBack}>
        <ArrowLeftIcon data-icon="inline-start" aria-hidden />
        Back
      </Button>
      <Button type="button" disabled={!canCreate || busy} onClick={onCreate}>
        <PlusIcon data-icon="inline-start" aria-hidden />
        {creating ? "Creating…" : "Create"}
      </Button>
    </DialogFooter>
  );
}
