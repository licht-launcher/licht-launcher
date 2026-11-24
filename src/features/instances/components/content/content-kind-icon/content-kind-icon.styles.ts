export const contentKindIconStyles = {
  well: "flex shrink-0 items-center justify-center rounded-md",
  sizes: {
    default: { well: "size-8", icon: "size-4" },
    sm: { well: "size-6", icon: "size-3.5" },
  },
  muted: { well: "bg-muted", icon: "text-muted-foreground" },
  kinds: {
    mods: { well: "bg-accent-violet/15", icon: "text-accent-violet" },
    resourcePacks: {
      well: "bg-accent-orange/15",
      icon: "text-accent-orange",
    },
    shaderPacks: { well: "bg-accent-teal/15", icon: "text-accent-teal" },
  },
} as const;
