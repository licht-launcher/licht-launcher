import type { ModpackCategory } from "@/lib/generated/ModpackCategory";

/**
 * One theme accent per category, so they read apart at a glance. The hover
 * classes keep the tone when it colors a button (such as a pressed filter),
 * overriding the button's own hover.
 */
export const CATEGORY_TONES = {
  adventure:
    "bg-accent-green/15 text-accent-green hover:bg-accent-green/25 hover:text-accent-green dark:hover:bg-accent-green/25",
  challenging:
    "bg-accent-rose/15 text-accent-rose hover:bg-accent-rose/25 hover:text-accent-rose dark:hover:bg-accent-rose/25",
  combat:
    "bg-accent-orange/15 text-accent-orange hover:bg-accent-orange/25 hover:text-accent-orange dark:hover:bg-accent-orange/25",
  kitchenSink:
    "bg-accent-yellow/15 text-accent-yellow hover:bg-accent-yellow/25 hover:text-accent-yellow dark:hover:bg-accent-yellow/25",
  lightweight:
    "bg-accent-teal/15 text-accent-teal hover:bg-accent-teal/25 hover:text-accent-teal dark:hover:bg-accent-teal/25",
  magic:
    "bg-accent-violet/15 text-accent-violet hover:bg-accent-violet/25 hover:text-accent-violet dark:hover:bg-accent-violet/25",
  multiplayer:
    "bg-accent-cool/15 text-accent-cool hover:bg-accent-cool/25 hover:text-accent-cool dark:hover:bg-accent-cool/25",
  optimization:
    "bg-accent-lime/15 text-accent-lime hover:bg-accent-lime/25 hover:text-accent-lime dark:hover:bg-accent-lime/25",
  quests:
    "bg-accent-pink/15 text-accent-pink hover:bg-accent-pink/25 hover:text-accent-pink dark:hover:bg-accent-pink/25",
  technology:
    "bg-accent-indigo/15 text-accent-indigo hover:bg-accent-indigo/25 hover:text-accent-indigo dark:hover:bg-accent-indigo/25",
} satisfies Record<ModpackCategory, string>;

export const categoryBadgeStyles = {
  tones: CATEGORY_TONES,
} as const;
