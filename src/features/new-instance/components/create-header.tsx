import { PlusIcon, WrenchIcon, XIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  DialogClose,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type { CreateView } from "@/features/new-instance/model/create-state";

export function CreateHeader({ view }: { view: CreateView }) {
  const Icon = view === "source" ? PlusIcon : WrenchIcon;

  return (
    <DialogHeader className="flex-row items-center gap-4 border-b border-border/40 pb-5">
      <span
        data-slot="create-header-icon"
        className="flex size-11 shrink-0 items-center justify-center rounded-xl bg-instance/20 text-instance"
      >
        <Icon aria-hidden className="size-5" strokeWidth={2} />
      </span>
      <div className="flex min-w-0 flex-1 items-center justify-between gap-2">
        <DialogTitle className="text-lg leading-tight">
          Create instance
        </DialogTitle>
        <DialogClose asChild>
          <Button type="button" variant="ghost" size="icon-sm" className="-my-1">
            <XIcon aria-hidden />
            <span className="sr-only">Close</span>
          </Button>
        </DialogClose>
      </div>
    </DialogHeader>
  );
}
