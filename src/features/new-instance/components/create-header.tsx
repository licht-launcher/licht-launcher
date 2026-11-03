import { DialogHeader, DialogTitle } from "@/components/ui/dialog";

/**
 * A full-width band like the form footer. The close button is the dialog's own
 * corner button, the same as in the other dialogs.
 */
export function CreateHeader() {
  return (
    <DialogHeader className="-mx-4 -mt-6 rounded-t-xl border-b px-4 py-5 pr-12">
      <DialogTitle className="text-lg leading-tight pt-0.5">
        Create instance
      </DialogTitle>
    </DialogHeader>
  );
}
