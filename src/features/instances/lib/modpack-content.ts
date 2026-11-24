import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";
import type { InstanceContentKind } from "@/lib/generated/InstanceContentKind";
import type { InstanceSource } from "@/lib/generated/InstanceSource";

/** Where a modpack instance came from: a store project or a file from disk. */
export type ModpackOrigin = Extract<
  InstanceSource,
  { type: "modpack" } | { type: "modpackFile" }
>;

export type StoreModpackOrigin = Extract<ModpackOrigin, { type: "modpack" }>;
export type FileModpackOrigin = Extract<ModpackOrigin, { type: "modpackFile" }>;

export function modpackOrigin(
  source: InstanceSource | undefined,
): ModpackOrigin | null {
  return source?.type === "modpack" || source?.type === "modpackFile"
    ? source
    : null;
}

const KIND_LABELS: Record<InstanceContentKind, [string, string]> = {
  mods: ["mod", "mods"],
  resourcePacks: ["resource pack", "resource packs"],
  shaderPacks: ["shader pack", "shader packs"],
};

const KIND_ORDER: readonly InstanceContentKind[] = [
  "mods",
  "resourcePacks",
  "shaderPacks",
];

export type ContentCount = { kind: InstanceContentKind; label: string };

/** Non-zero counts per kind, e.g. `47 mods` and `3 resource packs`. */
export function countContent(
  entries: readonly InstanceContentEntry[],
): ContentCount[] {
  return KIND_ORDER.flatMap((kind) => {
    const count = entries.filter((entry) => entry.kind === kind).length;
    if (count === 0) return [];
    const [singular, plural] = KIND_LABELS[kind];
    return [{ kind, label: `${count} ${count === 1 ? singular : plural}` }];
  });
}
