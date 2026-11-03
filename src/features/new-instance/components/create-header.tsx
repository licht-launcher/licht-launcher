import { PlusIcon, XIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import { DialogClose, DialogHeader, DialogTitle } from "@/components/ui/dialog";

/** A full-width band like the form footer, with the close button at the corner. */
export function CreateHeader() {
  return (
    <DialogHeader className="-mx-6 -mt-6 flex-row items-center gap-4 rounded-t-xl border-b bg-muted/50 py-4 pr-3 pl-6">
      <span
        data-slot="create-header-icon"
        className="flex size-11 shrink-0 items-center justify-center rounded-xl bg-instance/20 text-instance"
      >
        <PlusIcon aria-hidden className="size-5" strokeWidth={2} />
      </span>
      <div className="flex min-w-0 flex-1 items-center justify-between gap-2">
        <DialogTitle className="text-lg leading-tight">
          Create instance
        </DialogTitle>
        <DialogClose asChild>
          <Button type="button" variant="ghost" size="icon-sm">
            <XIcon aria-hidden />
            <span className="sr-only">Close</span>
          </Button>
        </DialogClose>
      </div>
    </DialogHeader>
  );
}
