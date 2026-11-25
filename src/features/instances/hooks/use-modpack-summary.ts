import { useState } from "react";
import {
  useModpack,
  useModpackVersions,
} from "@/features/discover/hooks/use-modpacks";
import type {
  FileModpackOrigin,
  StoreModpackOrigin,
} from "@/features/instances/lib/modpack-content";
import type { LinkedModpack } from "@/lib/generated/LinkedModpack";
import { SOURCE_LABELS } from "@/lib/modpack-sources";
import { formatTimeAgo } from "@/lib/time-ago";

/** What the launcher can tell about the modpack an instance is linked to. */
export type ModpackSummary = {
  name: string;
  version: string | null;
  /** How long ago the installed version was released, e.g. `3 weeks ago`. */
  updated: string | null;
  /** Provider and source, in reading order. */
  details: readonly string[];
};

function provider(author: string | null, sourceLabel: string): string {
  return author ? `by ${author} · ${sourceLabel}` : sourceLabel;
}

/**
 * The pack's name, version and provider. Details saved at install are shown
 * right away; only instances without them ask the store.
 */
export function useStoreModpackSummary(
  origin: StoreModpackOrigin,
  linked: LinkedModpack | null,
): ModpackSummary {
  const [now] = useState(() => Date.now());
  const fetch = linked === null;
  const modpack = useModpack(origin.source, origin.projectId, fetch);
  const versions = useModpackVersions(origin.source, origin.projectId, fetch);
  const sourceLabel = SOURCE_LABELS[origin.source];
  if (linked !== null) {
    return {
      name: linked.name,
      version: linked.versionName,
      updated: formatTimeAgo(linked.publishedAt, now),
      details: [provider(linked.author, sourceLabel)],
    };
  }
  const version = versions.data?.find((item) => item.id === origin.versionId);
  const details = modpack.isPending
    ? ["Loading modpack details…"]
    : modpack.isError
      ? ["Modpack details are unavailable."]
      : [provider(modpack.data.author || null, sourceLabel)];
  return {
    name: modpack.data?.name ?? "Modpack",
    version: version?.name ?? null,
    updated: modpack.isSuccess
      ? formatTimeAgo(version?.publishedAt ?? null, now)
      : null,
    details,
  };
}

/**
 * A pack imported from disk has no store page; what its archive declared at
 * import, when anything, stands in for it.
 */
export function fileModpackSummary(
  origin: FileModpackOrigin,
  linked: LinkedModpack | null,
): ModpackSummary {
  const installed = `Installed from a ${SOURCE_LABELS[origin.source]} file`;
  return {
    name: linked?.name ?? "Imported modpack",
    version: linked?.versionName ?? null,
    updated: null,
    details: linked?.author ? [`by ${linked.author}`, installed] : [installed],
  };
}
