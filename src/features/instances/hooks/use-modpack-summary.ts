import { useState } from "react";
import {
  useModpack,
  useModpackVersions,
} from "@/features/discover/hooks/use-modpacks";
import { SOURCE_LABELS } from "@/features/discover/lib/discover-search";
import { formatTimeAgo } from "@/features/discover/lib/format";
import type {
  FileModpackOrigin,
  StoreModpackOrigin,
} from "@/features/instances/lib/modpack-content";

/** What the launcher can tell about the modpack an instance is linked to. */
export type ModpackSummary = {
  name: string;
  iconUrl: string | null;
  version: string | null;
  /** How long ago the installed version was released, e.g. `3 weeks ago`. */
  updated: string | null;
  /** Provider and source, in reading order. */
  details: readonly string[];
};

/** The pack's name, version and provider from its store page. */
export function useStoreModpackSummary(
  origin: StoreModpackOrigin,
): ModpackSummary {
  const [now] = useState(() => Date.now());
  const modpack = useModpack(origin.source, origin.projectId);
  const versions = useModpackVersions(origin.source, origin.projectId);
  const version = versions.data?.find((item) => item.id === origin.versionId);
  const updated = formatTimeAgo(version?.publishedAt ?? null, now);
  const sourceLabel = SOURCE_LABELS[origin.source];
  const details = modpack.isPending
    ? ["Loading modpack details…"]
    : modpack.isError
      ? ["Modpack details are unavailable."]
      : [
          modpack.data.author
            ? `by ${modpack.data.author} · ${sourceLabel}`
            : sourceLabel,
        ];
  return {
    name: modpack.data?.name ?? "Modpack",
    iconUrl: modpack.data?.iconUrl ?? origin.iconUrl,
    version: version?.name ?? null,
    updated: modpack.isSuccess ? updated : null,
    details,
  };
}

/** A pack imported from disk has no store page to describe it. */
export function fileModpackSummary(origin: FileModpackOrigin): ModpackSummary {
  return {
    name: "Imported modpack",
    iconUrl: null,
    version: null,
    updated: null,
    details: [`Installed from a ${SOURCE_LABELS[origin.source]} file`],
  };
}
