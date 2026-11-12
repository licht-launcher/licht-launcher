import type { ComponentProps } from "react";

import { cn } from "@/lib/utils";

import { badgeStyles } from "./badge.styles";

/** A small, non-interactive label such as a category or tag. */
export function Badge({ className, ...props }: ComponentProps<"span">) {
  return (
    <span
      data-slot="badge"
      className={cn(badgeStyles.root, className)}
      {...props}
    />
  );
}
