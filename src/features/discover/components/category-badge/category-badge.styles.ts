export const categoryBadgeStyles = {
  // The line box is taller than the badge padding, and the icon box sits a
  // pixel above the cap height. py-0 lets the glyphs in; the nudge seats the icon.
  root: "border-border py-0 [&>svg]:translate-y-px",
} as const;
