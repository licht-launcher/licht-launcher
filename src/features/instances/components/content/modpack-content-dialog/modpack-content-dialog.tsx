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
import { ContentFilterSelect } from "@/features/instances/components/content/content-filter-select";
import { ContentKindIcon } from "@/features/instances/components/content/content-kind-icon";
import type { ContentActions } from "@/features/instances/components/content/content-row";
import { ContentTable } from "@/features/instances/components/content/content-table";
import {
  ContentTableColumns,
  ContentTableHead,
} from "@/features/instances/components/content/content-table-head";
import { useContentVirtualizer } from "@/features/instances/hooks/use-content-virtualizer";
import {
  fileModpackSummary,
  type ModpackSummary,
  useStoreModpackSummary,
} from "@/features/instances/hooks/use-modpack-summary";
import {
  type ContentFilter,
  type ContentSortDir,
  filterContentEntries,
  sortContentEntries,
} from "@/features/instances/lib/content-table";
import {
  countContent,
  type ModpackOrigin,
  type StoreModpackOrigin,
} from "@/features/instances/lib/modpack-content";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";

import { modpackContentDialogStyles as styles } from "./modpack-content-dialog.styles";

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
  origin,
  ...content
}: ModpackContent & {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  origin: ModpackOrigin;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className={styles.content} showCloseButton={false}>
        <DialogHeader className={styles.header}>
          {origin.type === "modpack" ? (
            <StoreModpackHeading origin={origin} />
          ) : (
            <ModpackHeading summary={fileModpackSummary(origin)} />
          )}
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

function StoreModpackHeading({ origin }: { origin: StoreModpackOrigin }) {
  return <ModpackHeading summary={useStoreModpackSummary(origin)} />;
}

/** The pack's icon, then which pack and version, who provides it and how recent it is. */
function ModpackHeading({ summary }: { summary: ModpackSummary }) {
  return (
    <>
      {summary.iconUrl ? (
        <img src={summary.iconUrl} alt="" className={styles.icon} />
      ) : (
        <span aria-hidden className={styles.iconFallback}>
          <PackageIcon />
        </span>
      )}
      <div className={styles.heading}>
        <DialogTitle className={styles.title}>Modpack content</DialogTitle>
        <DialogDescription className={styles.description}>
          {[
            summary.version
              ? `${summary.name} ${summary.version}`
              : summary.name,
            ...summary.details,
          ].join(" · ")}
        </DialogDescription>
      </div>
    </>
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
  return (
    <>
      <p className={styles.summary}>
        {counts.map((count) => (
          <span key={count.kind} className={styles.count}>
            <ContentKindIcon kind={count.kind} size="sm" tone="muted" />
            {count.label}
          </span>
        ))}
      </p>
      <div className={styles.toolbar}>
        <Input
          aria-label="Search modpack content"
          className={styles.searchInput}
          placeholder={`Search ${entries.length} files`}
          value={search}
          startIcon={<SearchIcon aria-hidden />}
          onChange={(event) => setSearch(event.target.value)}
        />
        <ContentFilterSelect value={filter} onValueChange={setFilter} />
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
    </>
  );
}
