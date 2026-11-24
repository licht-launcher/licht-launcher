import { useMemo } from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { ContentTable } from "@/features/instances/components/content/content-table";
import { useContentVirtualizer } from "@/features/instances/hooks/use-content-virtualizer";
import { sortContentEntries } from "@/features/instances/lib/content-table";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";

import { modpackContentDialogStyles as styles } from "./modpack-content-dialog.styles";

type ModpackContent = {
  folder: string;
  entries: InstanceContentEntry[];
};

/**
 * Lists the files the instance's modpack installed. They are read-only: the
 * modpack manages them while the instance is linked to it.
 */
export function ModpackContentDialog({
  open,
  onOpenChange,
  ...list
}: ModpackContent & {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className={styles.content}>
        <DialogHeader className={styles.header}>
          <DialogTitle>Modpack content</DialogTitle>
          <DialogDescription>
            Content installed by this instance's modpack.
          </DialogDescription>
        </DialogHeader>
        <ModpackContentList {...list} />
      </DialogContent>
    </Dialog>
  );
}

function ModpackContentList({ folder, entries }: ModpackContent) {
  const sorted = useMemo(() => sortContentEntries(entries, "asc"), [entries]);
  const virtualization = useContentVirtualizer({
    entries: sorted,
    folder,
    filter: "all",
    search: "",
    sortKey: "name",
    sortDir: "asc",
    enabled: true,
    disabled: false,
    removalOpen: false,
  });
  return (
    <ContentTable
      entries={sorted}
      totalEntries={sorted.length}
      virtualization={virtualization}
    />
  );
}
