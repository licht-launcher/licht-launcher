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

type ContentActions = {
  folder: string;
  entries: InstanceContentEntry[];
  disabled: boolean;
  removalOpen: boolean;
  onToggle: (entry: InstanceContentEntry, enabled: boolean) => void;
  onRemove: (entry: InstanceContentEntry) => void;
};

/** Lists the files the instance's modpack installed, with the same controls as the content list. */
export function ModpackContentDialog({
  open,
  onOpenChange,
  ...list
}: ContentActions & {
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

function ModpackContentList({
  folder,
  entries,
  disabled,
  removalOpen,
  onToggle,
  onRemove,
}: ContentActions) {
  const sorted = useMemo(() => sortContentEntries(entries, "asc"), [entries]);
  const virtualization = useContentVirtualizer({
    entries: sorted,
    folder,
    filter: "all",
    search: "",
    sortKey: "name",
    sortDir: "asc",
    enabled: true,
    disabled,
    removalOpen,
  });
  return (
    <ContentTable
      entries={sorted}
      totalEntries={sorted.length}
      virtualization={virtualization}
      disabled={disabled}
      onToggle={onToggle}
      onRemove={onRemove}
    />
  );
}
