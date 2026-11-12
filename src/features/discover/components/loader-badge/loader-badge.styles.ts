export const loaderBadgeStyles = {
  root: "font-semibold",
  icon: "size-3",
  /** One theme accent per loader, so they read apart at a glance. */
  tones: {
    fabric: "bg-accent-cool/15 text-accent-cool",
    forge: "bg-accent-rose/15 text-accent-rose",
    neoforge: "bg-accent-orange/15 text-accent-orange",
    quilt: "bg-accent-violet/15 text-accent-violet",
  },
} as const;
