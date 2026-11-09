import { Dialog, DialogContent } from "@/components/ui/dialog";
import { CreateFlow } from "@/features/new-instance/components/create-flow";

import { createInstanceDialogStyles } from "./create-instance-dialog.styles";

export function CreateInstanceDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        aria-describedby={undefined}
        className={createInstanceDialogStyles.content}
      >
        {/* Unmounted on close, so every opening starts from the sources. */}
        <CreateFlow
          onClose={() => {
            onOpenChange(false);
          }}
        />
      </DialogContent>
    </Dialog>
  );
}
