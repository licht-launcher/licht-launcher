import { useQuery } from "@tanstack/react-query";
import {
  ArrowDownIcon,
  ArrowUpIcon,
  ChevronRightIcon,
  CornerLeftUpIcon,
  FileIcon,
  FolderIcon,
  FolderOpenIcon,
} from "lucide-react";
import { useState } from "react";

import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Spinner } from "@/components/ui/spinner";
import {
  type FilesSortDir,
  type FilesSortKey,
  formatFileCreated,
  formatFileSize,
  sortFileEntries,
  toggleFilesSort,
} from "@/features/instances/lib/files-table";
import { listInstanceEntries } from "@/lib/commands";
import type { InstanceFsEntry } from "@/lib/generated/InstanceFsEntry";
import { cn } from "@/lib/utils";

function joinRelative(base: string, name: string): string {
  return base === "" ? name : `${base}/${name}`;
}

function parentRelative(path: string): string {
  const index = path.lastIndexOf("/");
  if (index < 0) {
    return "";
  }
  return path.slice(0, index);
}

function breadcrumbParts(relativePath: string): string[] {
  if (relativePath === "") {
    return [];
  }
  return relativePath.split("/");
}

export function FilesBreadcrumb({
  folder,
  relativePath,
  onRelativePathChange,
}: {
  folder: string | null;
  relativePath: string;
  onRelativePathChange: (path: string) => void;
}) {
  const parts = breadcrumbParts(relativePath);

  return (
    <nav aria-label="Current folder" className="min-w-0 flex-1">
      <ol className="flex min-w-0 flex-wrap items-center gap-1 text-xs font-medium text-muted-foreground">
        <li className="min-w-0">
          <button
            type="button"
            className={cn(
              "cursor-pointer rounded-sm outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring",
              relativePath === "" && "text-foreground",
            )}
            disabled={folder === null || relativePath === ""}
            onClick={() => {
              onRelativePathChange("");
            }}
          >
            Root
          </button>
        </li>
        {parts.map((part, index) => {
          const path = parts.slice(0, index + 1).join("/");
          const current = index === parts.length - 1;
          return (
            <li key={path} className="flex min-w-0 items-center gap-1">
              <ChevronRightIcon
                aria-hidden
                className="size-3.5 shrink-0"
                strokeWidth={2}
              />
              <button
                type="button"
                className={cn(
                  "cursor-pointer truncate rounded-sm outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring",
                  current && "text-foreground",
                )}
                disabled={current}
                onClick={() => {
                  onRelativePathChange(path);
                }}
              >
                {part}
              </button>
            </li>
          );
        })}
      </ol>
    </nav>
  );
}

export function FilesSection({
  folder,
  relativePath,
  onRelativePathChange,
}: {
  folder: string | null;
  relativePath: string;
  onRelativePathChange: (path: string) => void;
}) {
  const [sortKey, setSortKey] = useState<FilesSortKey>("name");
  const [sortDir, setSortDir] = useState<FilesSortDir>("asc");
  const listing = useQuery({
    queryKey: ["instance-entries", folder, relativePath],
    queryFn: () => listInstanceEntries(folder ?? "", relativePath),
    enabled: folder !== null,
  });
  const entries = sortFileEntries(listing.data ?? [], sortKey, sortDir);

  function onSort(nextKey: FilesSortKey) {
    const next = toggleFilesSort(sortKey, sortDir, nextKey);
    setSortKey(next.key);
    setSortDir(next.dir);
  }

  const showTable =
    folder !== null &&
    !listing.isPending &&
    !listing.isError &&
    ((listing.data?.length ?? 0) > 0 || relativePath !== "");

  return (
    <section
      aria-label="Instance files"
      className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border border-border/60 bg-black/20"
    >
      {folder === null ? (
        <p className="p-3 text-sm leading-normal text-muted-foreground">
          Select an instance to browse its files.
        </p>
      ) : listing.isPending ? (
        <div className="flex items-center gap-2 p-3 text-sm leading-normal text-muted-foreground">
          <Spinner className="size-4" />
          Loading…
        </div>
      ) : listing.isError ? (
        <div className="flex flex-col gap-2 p-3">
          <p className="text-sm leading-normal text-destructive">
            Could not list this folder.
          </p>
          {relativePath !== "" ? (
            <Button
              type="button"
              variant="secondary"
              size="sm"
              className="self-start"
              onClick={() => {
                onRelativePathChange(parentRelative(relativePath));
              }}
            >
              Go up
            </Button>
          ) : null}
        </div>
      ) : (
        <>
          {showTable ? (
            <table className="w-full shrink-0 table-fixed caption-bottom text-sm leading-normal">
              <colgroup>
                <col />
                <col className="w-24" />
                <col className="w-44" />
              </colgroup>
              <FilesTableHead
                sortKey={sortKey}
                sortDir={sortDir}
                onSort={onSort}
              />
            </table>
          ) : null}
          <ScrollArea className="min-h-0 flex-1">
            {(listing.data?.length ?? 0) === 0 ? (
              <div className="flex flex-col">
                {relativePath !== "" ? (
                  <table className="w-full table-fixed caption-bottom text-sm leading-normal">
                    <colgroup>
                      <col />
                      <col className="w-24" />
                      <col className="w-44" />
                    </colgroup>
                    <tbody>
                      <ParentRow
                        onOpen={() => {
                          onRelativePathChange(parentRelative(relativePath));
                        }}
                      />
                    </tbody>
                  </table>
                ) : null}
                <EmptyFolderState />
              </div>
            ) : (
              <table className="w-full table-fixed caption-bottom text-sm leading-normal">
                <colgroup>
                  <col />
                  <col className="w-24" />
                  <col className="w-44" />
                </colgroup>
                <tbody>
                  {relativePath !== "" ? (
                    <ParentRow
                      onOpen={() => {
                        onRelativePathChange(parentRelative(relativePath));
                      }}
                    />
                  ) : null}
                  {entries.map((entry, index) => (
                    <FileRow
                      key={`${entry.isDir ? "d" : "f"}:${entry.name}`}
                      entry={entry}
                      striped={index % 2 === 1}
                      onOpen={() => {
                        if (entry.isDir) {
                          onRelativePathChange(
                            joinRelative(relativePath, entry.name),
                          );
                        }
                      }}
                    />
                  ))}
                </tbody>
              </table>
            )}
          </ScrollArea>
        </>
      )}
    </section>
  );
}

function FilesTableHead({
  sortKey,
  sortDir,
  onSort,
}: {
  sortKey: FilesSortKey;
  sortDir: FilesSortDir;
  onSort: (key: FilesSortKey) => void;
}) {
  return (
    <thead>
      <tr className="border-b border-border/60 text-left text-xs font-medium tracking-wide text-muted-foreground">
        <SortableHeader
          label="Name"
          column="name"
          sortKey={sortKey}
          sortDir={sortDir}
          onSort={onSort}
          className="px-3 py-3"
        />
        <SortableHeader
          label="Size"
          column="size"
          sortKey={sortKey}
          sortDir={sortDir}
          onSort={onSort}
          className="w-24 px-3 py-3"
        />
        <SortableHeader
          label="Created"
          column="created"
          sortKey={sortKey}
          sortDir={sortDir}
          onSort={onSort}
          className="w-44 px-3 py-3"
        />
      </tr>
    </thead>
  );
}

function SortableHeader({
  label,
  column,
  sortKey,
  sortDir,
  onSort,
  className,
}: {
  label: string;
  column: FilesSortKey;
  sortKey: FilesSortKey;
  sortDir: FilesSortDir;
  onSort: (key: FilesSortKey) => void;
  className?: string;
}) {
  const active = sortKey === column;
  const SortIcon = sortDir === "desc" ? ArrowDownIcon : ArrowUpIcon;

  return (
    <th className={className} aria-sort={active ? `${sortDir}ending` : "none"}>
      <button
        type="button"
        className={cn(
          "inline-flex cursor-pointer items-center gap-1 rounded-sm font-medium outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring",
          active && "text-foreground",
        )}
        onClick={() => {
          onSort(column);
        }}
      >
        {label}
        {active ? (
          <SortIcon
            aria-hidden
            className="size-3 shrink-0 text-muted-foreground opacity-60"
            strokeWidth={2}
          />
        ) : null}
      </button>
    </th>
  );
}

function EmptyFolderState() {
  return (
    <div className="flex flex-col items-center justify-center gap-3 px-3 py-16 text-muted-foreground">
      <span className="flex size-14 shrink-0 items-center justify-center rounded-full bg-muted">
        <FolderOpenIcon
          aria-hidden
          className="size-6 shrink-0"
          strokeWidth={1.5}
        />
      </span>
      <p className="font-brand text-sm font-medium leading-normal">
        This folder is empty.
      </p>
    </div>
  );
}

function ParentRow({ onOpen }: { onOpen: () => void }) {
  return (
    <tr className="border-b border-border/60 transition-colors hover:bg-muted/40">
      <td className="px-3 py-3">
        <button
          type="button"
          aria-label="Up"
          className="flex w-full min-w-0 cursor-pointer items-center gap-2 text-left outline-none focus-visible:ring-2 focus-visible:ring-ring"
          onClick={onOpen}
          onDoubleClick={onOpen}
        >
          <CornerLeftUpIcon
            aria-hidden
            className="size-4 shrink-0 text-muted-foreground"
            strokeWidth={2}
          />
          <span className="min-w-0 truncate font-medium text-foreground">
            ..
          </span>
        </button>
      </td>
      <td className="px-3 py-3 text-xs text-muted-foreground tabular-nums">
        —
      </td>
      <td className="px-3 py-3 text-xs text-muted-foreground tabular-nums">
        —
      </td>
    </tr>
  );
}

function FileRow({
  entry,
  striped,
  onOpen,
}: {
  entry: InstanceFsEntry;
  striped: boolean;
  onOpen: () => void;
}) {
  const Icon = entry.isDir ? FolderIcon : FileIcon;
  return (
    <tr
      className={cn(
        "border-b border-border/60 transition-colors last:border-b-0",
        striped && "bg-black/15",
        entry.isDir && "cursor-pointer hover:bg-muted/40",
        !entry.isDir && "hover:bg-muted/30",
      )}
      onClick={() => {
        if (entry.isDir) {
          onOpen();
        }
      }}
      onDoubleClick={() => {
        if (entry.isDir) {
          onOpen();
        }
      }}
      onKeyDown={(event) => {
        if (entry.isDir && event.key === "Enter") {
          onOpen();
        }
      }}
      tabIndex={entry.isDir ? 0 : undefined}
      role={entry.isDir ? "button" : undefined}
      aria-label={entry.isDir ? `${entry.name}, folder` : `${entry.name}, file`}
    >
      <td className="px-3 py-3">
        <span className="flex min-w-0 items-center gap-2">
          <Icon
            aria-hidden
            className="size-4 shrink-0 text-muted-foreground"
            strokeWidth={2}
          />
          <span className="min-w-0 truncate font-medium text-foreground">
            {entry.name}
          </span>
        </span>
      </td>
      <td className="px-3 py-3 text-xs text-muted-foreground tabular-nums">
        {formatFileSize(entry.size)}
      </td>
      <td className="px-3 py-3 text-xs text-muted-foreground tabular-nums">
        {formatFileCreated(entry.created)}
      </td>
    </tr>
  );
}
