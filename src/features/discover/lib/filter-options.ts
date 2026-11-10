import {
  CompassIcon,
  CpuIcon,
  FeatherIcon,
  GaugeIcon,
  type LucideIcon,
  PackageIcon,
  ScrollTextIcon,
  SkullIcon,
  SparklesIcon,
  SwordsIcon,
  UsersIcon,
} from "lucide-react";

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
export const CATEGORY_OPTIONS: readonly (Option<ModpackCategory> & {
  icon: LucideIcon;
})[] = [
  { value: "adventure", label: "Adventure", icon: CompassIcon },
  { value: "challenging", label: "Challenging", icon: SkullIcon },
  { value: "combat", label: "Combat", icon: SwordsIcon },
  { value: "kitchenSink", label: "Kitchen sink", icon: PackageIcon },
  { value: "lightweight", label: "Lightweight", icon: FeatherIcon },
  { value: "magic", label: "Magic", icon: SparklesIcon },
  { value: "multiplayer", label: "Multiplayer", icon: UsersIcon },
  { value: "optimization", label: "Optimization", icon: GaugeIcon },
  { value: "quests", label: "Quests", icon: ScrollTextIcon },
  { value: "technology", label: "Technology", icon: CpuIcon },
];
