import { DialogHeader, DialogTitle } from "@/components/ui/dialog";

/**
 * A full-width band like the form footer. The close button is the dialog's own
 * corner button, the same as in the other dialogs.
 */
export function CreateHeader() {
  return (
    <DialogHeader className="-mx-4 rounded-t-xl px-4.5 ">
      <DialogTitle className="text-lg leading-tight">
        Create instance
      </DialogTitle>
    </DialogHeader>
  );
}
