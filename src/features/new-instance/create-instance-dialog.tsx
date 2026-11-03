import { Dialog, DialogContent } from "@/components/ui/dialog";
import { CreateFlow } from "@/features/new-instance/components/create-flow";

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
        showCloseButton={false}
        aria-describedby={undefined}
        className="flex max-h-[calc(100%-2rem)] flex-col gap-5 overflow-hidden p-6 sm:max-w-xl"
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
