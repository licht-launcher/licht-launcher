import type { ModpackCategory } from "@/lib/generated/ModpackCategory";
import type { ModpackLoader } from "@/lib/generated/ModpackLoader";
import type { ModpackSort } from "@/lib/generated/ModpackSort";

type Option<T extends string> = { value: T; label: string };

export const SORT_OPTIONS: readonly Option<ModpackSort>[] = [
  { value: "downloads", label: "Downloads" },
  { value: "updated", label: "Recently updated" },
  { value: "relevance", label: "Relevance" },
];

export const LOADER_OPTIONS: readonly Option<ModpackLoader>[] = [
  { value: "fabric", label: "Fabric" },
  { value: "forge", label: "Forge" },
  { value: "neoforge", label: "NeoForge" },
  { value: "quilt", label: "Quilt" },
];

/** Licht's shared categories, in the order the sidebar lists them. */
export const CATEGORY_OPTIONS: readonly Option<ModpackCategory>[] = [
  { value: "adventure", label: "Adventure" },
  { value: "challenging", label: "Challenging" },
  { value: "combat", label: "Combat" },
  { value: "kitchenSink", label: "Kitchen sink" },
  { value: "lightweight", label: "Lightweight" },
  { value: "magic", label: "Magic" },
  { value: "multiplayer", label: "Multiplayer" },
  { value: "optimization", label: "Optimization" },
  { value: "quests", label: "Quests" },
  { value: "technology", label: "Technology" },
];
