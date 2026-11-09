import { useQuery } from "@tanstack/react-query";
import {
  ArrowDownAZIcon,
  ArrowDownIcon,
  ArrowUpIcon,
  BinaryIcon,
  ChevronRightIcon,
  ClockIcon,
  CornerLeftUpIcon,
  FileIcon,
  FolderIcon,
  FolderOpenIcon,
  type LucideIcon,
} from "lucide-react";
import { useState } from "react";
import { listInstanceEntries } from "@/api/commands";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  type FilesSortDir,
  type FilesSortKey,
  formatFileCreated,
  formatFileSize,
  sortFileEntries,
  toggleFilesSort,
} from "@/features/instances/lib/files-table";
import type { InstanceFsEntry } from "@/lib/generated/InstanceFsEntry";
import { cn } from "@/lib/utils";

import { filesSectionStyles } from "./files-section.styles";

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

const filesSkeletonRows = ["one", "two", "three", "four", "five", "six"];
const filesSkeletonOpacity = [
  "opacity-100",
  "opacity-85",
  "opacity-70",
  "opacity-55",
  "opacity-40",
  "opacity-25",
];

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
    <nav
      aria-label="Current folder"
      className={filesSectionStyles.breadcrumbWrapper}
    >
      <ol className={filesSectionStyles.breadcrumbs}>
        <li className={filesSectionStyles.breadcrumbItem}>
          <button
            type="button"
            className={cn(
              filesSectionStyles.breadcrumbButton,
              relativePath === "" && filesSectionStyles.breadcrumbCurrent,
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
            <li key={path} className={filesSectionStyles.breadcrumbSegment}>
              <ChevronRightIcon
                aria-hidden
                className={filesSectionStyles.breadcrumbIcon}
                strokeWidth={2}
              />
              <button
                type="button"
                className={cn(
                  filesSectionStyles.breadcrumbLink,
                  current && filesSectionStyles.breadcrumbCurrent,
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
    placeholderData: (previous) => previous,
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
    <section aria-label="Instance files" className={filesSectionStyles.root}>
      {folder === null ? (
        <p className={filesSectionStyles.message}>
          Select an instance to browse its files.
        </p>
      ) : listing.isPending ? (
        <>
          <div className={filesSectionStyles.tableWrapper}>
            <table className={filesSectionStyles.table}>
              <colgroup>
                <col />
                <col className={filesSectionStyles.sizeColumn} />
                <col className={filesSectionStyles.dateColumn} />
              </colgroup>
              <FilesTableHead
                sortKey={sortKey}
                sortDir={sortDir}
                onSort={() => {}}
              />
            </table>
          </div>
          <Progress
            aria-label="Loading files"
            className={filesSectionStyles.progress}
          />
          <ScrollArea className={filesSectionStyles.scrollArea}>
            <div className={filesSectionStyles.tableWrapper}>
              <table className={filesSectionStyles.table}>
                <colgroup>
                  <col />
                  <col className={filesSectionStyles.sizeColumn} />
                  <col className={filesSectionStyles.dateColumn} />
                </colgroup>
                <tbody>
                  {filesSkeletonRows.map((row, index) => (
                    <tr key={row} className={filesSectionStyles.skeletonRow}>
                      <td className={filesSectionStyles.skeletonCell}>
                        <div
                          className={cn(
                            filesSectionStyles.skeletonItem,
                            filesSkeletonOpacity[index],
                          )}
                        >
                          <div className={filesSectionStyles.skeletonIcon} />
                          <div className={filesSectionStyles.skeletonName} />
                        </div>
                      </td>
                      <td className={filesSectionStyles.skeletonCell}>
                        <div
                          className={cn(
                            filesSectionStyles.skeletonSize,
                            filesSkeletonOpacity[index],
                          )}
                        />
                      </td>
                      <td className={filesSectionStyles.skeletonCell}>
                        <div
                          className={cn(
                            filesSectionStyles.skeletonDate,
                            filesSkeletonOpacity[index],
                          )}
                        />
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </ScrollArea>
        </>
      ) : listing.isError ? (
        <div className={filesSectionStyles.errorBlock}>
          <p className={filesSectionStyles.errorText}>
            Could not list this folder.
          </p>
          {relativePath !== "" ? (
            <Button
              type="button"
              variant="secondary"
              size="sm"
              className={filesSectionStyles.retryButton}
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
            <table className={filesSectionStyles.headTable}>
              <colgroup>
                <col />
                <col className={filesSectionStyles.sizeColumn} />
                <col className={filesSectionStyles.dateColumn} />
              </colgroup>
              <FilesTableHead
                sortKey={sortKey}
                sortDir={sortDir}
                onSort={onSort}
              />
            </table>
          ) : null}
          <ScrollArea className={filesSectionStyles.scrollArea}>
            {(listing.data?.length ?? 0) === 0 ? (
              <div className={filesSectionStyles.bodyWrapper}>
                {relativePath !== "" ? (
                  <table className={filesSectionStyles.bodyTable}>
                    <colgroup>
                      <col />
                      <col className={filesSectionStyles.sizeColumn} />
                      <col className={filesSectionStyles.dateColumn} />
                    </colgroup>
                    <tbody>
                      <ParentRow
                        striped
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
              <table className={filesSectionStyles.bodyTable}>
                <colgroup>
                  <col />
                  <col className={filesSectionStyles.sizeColumn} />
                  <col className={filesSectionStyles.dateColumn} />
                </colgroup>
                <tbody>
                  {relativePath !== "" ? (
                    <ParentRow
                      striped
                      onOpen={() => {
                        onRelativePathChange(parentRelative(relativePath));
                      }}
                    />
                  ) : null}
                  {entries.map((entry, index) => {
                    const rowIndex = relativePath !== "" ? index + 1 : index;
                    return (
                      <FileRow
                        key={`${entry.isDir ? "d" : "f"}:${entry.name}`}
                        entry={entry}
                        striped={rowIndex % 2 === 0}
                        onOpen={() => {
                          if (entry.isDir) {
                            onRelativePathChange(
                              joinRelative(relativePath, entry.name),
                            );
                          }
                        }}
                      />
                    );
                  })}
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
      <tr className={filesSectionStyles.headRow}>
        <SortableHeader
          label="Name"
          icon={ArrowDownAZIcon}
          column="name"
          sortKey={sortKey}
          sortDir={sortDir}
          onSort={onSort}
          className={filesSectionStyles.skeletonCell}
        />
        <SortableHeader
          label="Size"
          icon={BinaryIcon}
          column="size"
          sortKey={sortKey}
          sortDir={sortDir}
          onSort={onSort}
          className={filesSectionStyles.sizeHeader}
        />
        <SortableHeader
          label="Created"
          icon={ClockIcon}
          column="created"
          sortKey={sortKey}
          sortDir={sortDir}
          onSort={onSort}
          className={filesSectionStyles.dateHeader}
        />
      </tr>
    </thead>
  );
}

function SortableHeader({
  label,
  icon: Icon,
  column,
  sortKey,
  sortDir,
  onSort,
  className,
}: {
  label: string;
  icon: LucideIcon;
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
          filesSectionStyles.sortButton,
          active && filesSectionStyles.breadcrumbCurrent,
        )}
        onClick={() => {
          onSort(column);
        }}
      >
        <Icon
          aria-hidden
          className={filesSectionStyles.breadcrumbIcon}
          strokeWidth={2}
        />
        {label}
        {active ? (
          <SortIcon
            aria-hidden
            className={filesSectionStyles.sortIconIdle}
            strokeWidth={2}
          />
        ) : null}
      </button>
    </th>
  );
}

function EmptyFolderState() {
  return (
    <div className={filesSectionStyles.empty}>
      <span className={filesSectionStyles.emptyIconWell}>
        <FolderOpenIcon
          aria-hidden
          className={filesSectionStyles.emptyIcon}
          strokeWidth={1.5}
        />
      </span>
      <p className={filesSectionStyles.emptyText}>This folder is empty.</p>
    </div>
  );
}

function ParentRow({
  striped,
  onOpen,
}: {
  striped?: boolean;
  onOpen: () => void;
}) {
  return (
    <tr
      className={cn(
        filesSectionStyles.parentRow,
        striped && filesSectionStyles.rowStriped,
      )}
    >
      <td className={filesSectionStyles.skeletonCell}>
        <button
          type="button"
          aria-label="Up"
          className={filesSectionStyles.entryButton}
          onClick={onOpen}
          onDoubleClick={onOpen}
        >
          <CornerLeftUpIcon
            aria-hidden
            className={filesSectionStyles.entryIcon}
            strokeWidth={2}
          />
          <span className={filesSectionStyles.entryName}>..</span>
        </button>
      </td>
      <td className={filesSectionStyles.metaCell}>—</td>
      <td className={filesSectionStyles.metaCell}>—</td>
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
        filesSectionStyles.row,
        striped && filesSectionStyles.rowStriped,
        entry.isDir && filesSectionStyles.rowDirectory,
        !entry.isDir && filesSectionStyles.rowFile,
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
      <td className={filesSectionStyles.skeletonCell}>
        <span className={filesSectionStyles.entryContent}>
          <Icon
            aria-hidden
            className={filesSectionStyles.entryIcon}
            strokeWidth={2}
          />
          <span className={filesSectionStyles.entryName}>{entry.name}</span>
        </span>
      </td>
      <td className={filesSectionStyles.metaCell}>
        {formatFileSize(entry.size)}
      </td>
      <td className={filesSectionStyles.metaCell}>
        {formatFileCreated(entry.created)}
      </td>
    </tr>
  );
}
