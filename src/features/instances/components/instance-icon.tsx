import { BoxIcon } from "lucide-react";

import { cn } from "@/lib/utils";

const sizes = {
  sm: {
    frame: "size-6 rounded-md",
    icon: "size-3.5",
  },
  md: {
    frame: "size-10 rounded-md",
    icon: "size-5",
  },
} as const;

/** Placeholder tile for an instance; later can host a modpack image. */
export function InstanceIcon({
  size = "sm",
  className,
}: {
  size?: keyof typeof sizes;
  className?: string;
}) {
  const tokens = sizes[size];
  return (
    <span
      aria-hidden
      className={cn(
        "flex shrink-0 items-center justify-center bg-instance/15 text-instance",
        tokens.frame,
        className,
      )}
    >
      <BoxIcon className={tokens.icon} strokeWidth={2} />
    </span>
  );
}
