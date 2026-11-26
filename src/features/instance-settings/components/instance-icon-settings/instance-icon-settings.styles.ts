// Rows follow the launcher settings cards.
export const instanceIconSettingsStyles = {
  row: "flex items-center justify-between gap-3",
  rowText: "flex min-w-0 flex-col gap-0.5",
  rowLabel: "text-sm font-normal",
  rowDescription: "text-sm text-muted-foreground",
  trigger:
    "group/icon relative shrink-0 cursor-pointer rounded-lg outline-none ring-offset-2 ring-offset-background transition-shadow hover:ring-2 hover:ring-ring/60 focus-visible:ring-3 focus-visible:ring-ring/50 data-[state=open]:ring-2 data-[state=open]:ring-ring disabled:pointer-events-none disabled:opacity-50",
  // Darkens the image and shows a pencil, so the icon reads as editable.
  overlay:
    "pointer-events-none absolute inset-0 flex items-center justify-center rounded-lg bg-black/50 text-white opacity-0 transition-opacity group-hover/icon:opacity-100 group-focus-visible/icon:opacity-100 group-data-[state=open]/icon:opacity-100",
  overlayIcon: "size-4",
} as const;
