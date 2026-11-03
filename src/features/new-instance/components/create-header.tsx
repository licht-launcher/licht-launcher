import { DialogHeader, DialogTitle } from "@/components/ui/dialog";

/**
 * A full-width band like the form footer. The close button is the dialog's own
 * corner button, the same as in the other dialogs.
 */
export function CreateHeader() {
  return (
    <DialogHeader className="-mx-6 -mt-6 rounded-t-xl border-b bg-muted/50 px-6 py-3 pr-12">
      <DialogTitle className="text-lg leading-tight">
        Create instance
      </DialogTitle>
    </DialogHeader>
  );
}
