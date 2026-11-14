import type { ModpackLoader } from "@/lib/generated/ModpackLoader";

/**
 * One theme accent per loader, so they read apart at a glance. The hover
 * classes repeat the resting tone, so a pressed filter does not change when
 * the pointer is over it.
 */
export const LOADER_TONES = {
  fabric:
    "border-accent-cool/25 bg-accent-cool/15 text-accent-cool hover:border-accent-cool/25 hover:bg-accent-cool/15 hover:text-accent-cool dark:hover:border-accent-cool/25 dark:hover:bg-accent-cool/15",
  forge:
    "border-accent-rose/25 bg-accent-rose/15 text-accent-rose hover:border-accent-rose/25 hover:bg-accent-rose/15 hover:text-accent-rose dark:hover:border-accent-rose/25 dark:hover:bg-accent-rose/15",
  neoforge:
    "border-accent-orange/25 bg-accent-orange/15 text-accent-orange hover:border-accent-orange/25 hover:bg-accent-orange/15 hover:text-accent-orange dark:hover:border-accent-orange/25 dark:hover:bg-accent-orange/15",
  quilt:
    "border-accent-violet/25 bg-accent-violet/15 text-accent-violet hover:border-accent-violet/25 hover:bg-accent-violet/15 hover:text-accent-violet dark:hover:border-accent-violet/25 dark:hover:bg-accent-violet/15",
} satisfies Record<ModpackLoader, string>;

export const loaderBadgeStyles = {
  root: "font-semibold",
  icon: "size-3",
  tones: LOADER_TONES,
} as const;
