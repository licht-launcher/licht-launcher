import { BoxesIcon, FilePlusIcon, UploadIcon } from "lucide-react";
import type { ReactNode } from "react";

import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useUploadInstance } from "@/features/new-instance/hooks/use-upload-instance";

/**
 * The "Create instance" button's menu. Scratch opens the form popup, upload
 * opens the system picker right away, and the modpack option waits for Discover.
 * The trigger is the button passed as `children`.
 */
export function CreateInstanceMenu({
  onScratch,
  side = "bottom",
  children,
}: {
  onScratch: () => void;
  side?: "top" | "bottom";
  children: ReactNode;
}) {
  const { upload } = useUploadInstance();

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>{children}</DropdownMenuTrigger>
      <DropdownMenuContent
        align="start"
        side={side}
        aria-label="Create instance"
      >
        <DropdownMenuItem onSelect={onScratch}>
          <FilePlusIcon aria-hidden />
          From scratch
        </DropdownMenuItem>
        <DropdownMenuItem disabled>
          <BoxesIcon aria-hidden />
          From a modpack
        </DropdownMenuItem>
        <DropdownMenuItem
          disabled={upload.isPending}
          onSelect={() => {
            upload.mutate();
          }}
        >
          <UploadIcon aria-hidden />
          Upload a pack file
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
