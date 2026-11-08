import { DialogHeader, DialogTitle } from "@/components/ui/dialog";

import { createHeaderStyles } from "./create-header.styles";

/**
 * A full-width band like the form footer. The close button is the dialog's own
 * corner button, the same as in the other dialogs.
 */
export function CreateHeader() {
  return (
    <DialogHeader className={createHeaderStyles.root}>
      <DialogTitle className={createHeaderStyles.title}>
        Create instance
      </DialogTitle>
    </DialogHeader>
  );
}
