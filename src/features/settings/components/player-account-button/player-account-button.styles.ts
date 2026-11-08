export const playerAccountButtonStyles = {
  trigger:
    "h-7 max-w-48 justify-start gap-2 rounded-md px-2 text-xs text-muted-foreground data-[state=open]:border-foreground/20 data-[state=open]:bg-muted data-[state=open]:text-foreground",
  triggerName: "truncate",
  menu: "min-w-56 font-brand",
  menuMessage: "px-2 py-1.5 text-xs text-muted-foreground",
  menuError: "px-2 py-1.5 text-xs text-destructive",
} as const;
