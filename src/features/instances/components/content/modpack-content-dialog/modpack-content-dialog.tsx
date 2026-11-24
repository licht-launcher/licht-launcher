import { PackageIcon, SearchIcon, XIcon } from "lucide-react";
import { useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { ContentKindIcon } from "@/features/instances/components/content/content-kind-icon";
import type { ContentActions } from "@/features/instances/components/content/content-row";
import { ContentTable } from "@/features/instances/components/content/content-table";
import {
  ContentTableColumns,
  ContentTableHead,
} from "@/features/instances/components/content/content-table-head";
import { useContentVirtualizer } from "@/features/instances/hooks/use-content-virtualizer";
import {
  type ContentFilter,
  type ContentSortDir,
  filterContentEntries,
  sortContentEntries,
} from "@/features/instances/lib/content-table";
import { countContent } from "@/features/instances/lib/modpack-content";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";
import type { InstanceContentKind } from "@/lib/generated/InstanceContentKind";

import { modpackContentDialogStyles as styles } from "./modpack-content-dialog.styles";

const FILTER_LABELS: Record<InstanceContentKind, string> = {
  mods: "Mods",
  resourcePacks: "Resource Packs",
  shaderPacks: "Shader Packs",
};

type ModpackContent = {
  folder: string;
  entries: InstanceContentEntry[];
  /** The modpack's files can be disabled but not removed while the instance is linked. */
  actions: Omit<ContentActions, "onRemove">;
};

/** Browses the files the instance's modpack installed. */
export function ModpackContentDialog({
  open,
  onOpenChange,
  iconUrl,
  ...content
}: ModpackContent & {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  iconUrl: string | null;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className={styles.content} showCloseButton={false}>
        <DialogHeader className={styles.header}>
          {iconUrl ? (
            <img src={iconUrl} alt="" className={styles.icon} />
          ) : (
            <span aria-hidden className={styles.iconFallback}>
              <PackageIcon />
            </span>
          )}
          <DialogTitle className={styles.title}>Modpack content</DialogTitle>
          <DialogDescription className={styles.description}>
            Files installed by this instance's modpack.
          </DialogDescription>
          <DialogClose asChild>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              aria-label="Close"
            >
              <XIcon aria-hidden />
            </Button>
          </DialogClose>
        </DialogHeader>
        <ModpackContentBrowser {...content} />
      </DialogContent>
    </Dialog>
  );
}

/** Mounted only while the dialog is open, so search and filters start fresh each time. */
function ModpackContentBrowser({ folder, entries, actions }: ModpackContent) {
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState<ContentFilter>("all");
  const [sortDir, setSortDir] = useState<ContentSortDir>("asc");
  const counts = countContent(entries);
  const visible = useMemo(
    () =>
      sortContentEntries(
        filterContentEntries(entries, filter, search),
        sortDir,
      ),
    [entries, filter, search, sortDir],
  );
  const virtualization = useContentVirtualizer({
    entries: visible,
    folder,
    filter,
    search,
    sortKey: "name",
    sortDir,
    enabled: true,
    disabled: actions.disabled,
    removalOpen: false,
  });
  const filters: ContentFilter[] = [
    "all",
    ...counts.map((count) => count.kind),
  ];
  return (
    <>
      <div className={styles.toolbar}>
        <Input
          aria-label="Search modpack content"
          placeholder={`Search ${entries.length} files`}
          value={search}
          startIcon={<SearchIcon aria-hidden />}
          onChange={(event) => setSearch(event.target.value)}
        />
        <fieldset className={styles.filters}>
          <legend className={styles.legend}>Filter by type</legend>
          {filters.map((option) => (
            <Button
              key={option}
              type="button"
              size="sm"
              variant={filter === option ? "secondary" : "outline"}
              aria-pressed={filter === option}
              onClick={() => setFilter(option)}
            >
              {option === "all" ? "All" : FILTER_LABELS[option]}
            </Button>
          ))}
        </fieldset>
      </div>
      <div className={styles.list}>
        <table className={styles.headTable}>
          <ContentTableColumns />
          <ContentTableHead
            sortKey="name"
            sortDir={sortDir}
            onSort={() => setSortDir(sortDir === "asc" ? "desc" : "asc")}
          />
        </table>
        <ContentTable
          entries={visible}
          totalEntries={entries.length}
          virtualization={virtualization}
          actions={actions}
        />
      </div>
      <p className={styles.footer}>
        {counts.map((count) => (
          <span key={count.kind} className={styles.count}>
            <ContentKindIcon kind={count.kind} size="sm" />
            {count.label}
          </span>
        ))}
      </p>
    </>
  );
}
