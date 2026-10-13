import { convertFileSrc } from "@tauri-apps/api/core";

import grassFallback from "@/assets/instance-grass.svg";
import { cn } from "@/lib/utils";

const sizes = {
  sm: {
    frame: "size-6 rounded-md",
  },
  md: {
    frame: "size-10 rounded-md",
  },
} as const;

/** Instance tile: modpack `icon.png` when present, otherwise original grass-block art. */
export function InstanceIcon({
  iconPath = null,
  size = "sm",
  className,
}: {
  iconPath?: string | null;
  size?: keyof typeof sizes;
  className?: string;
}) {
  const tokens = sizes[size];
  const src =
    iconPath !== null && iconPath !== ""
      ? convertFileSrc(iconPath)
      : grassFallback;

  return (
    <span
      aria-hidden
      className={cn(
        "flex shrink-0 items-center justify-center overflow-hidden bg-muted",
        tokens.frame,
        className,
      )}
    >
      <img
        src={src}
        alt=""
        className="size-full object-cover"
        draggable={false}
      />
    </span>
  );
}
