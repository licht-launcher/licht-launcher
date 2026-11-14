import { convertFileSrc } from "@tauri-apps/api/core";

import grassFallback from "@/assets/instance-grass.svg?no-inline";
import { cn } from "@/lib/utils";

import { instanceIconStyles } from "./instance-icon.styles";

function assetSrc(iconPath: string): string | null {
  try {
    return convertFileSrc(iconPath);
  } catch {
    return null;
  }
}

/** Instance tile: modpack `icon.png` when present, otherwise original grass-block art. */
export function InstanceIcon({
  iconPath = null,
  size = "sm",
  className,
}: {
  iconPath?: string | null;
  size?: keyof typeof instanceIconStyles.sizes;
  className?: string;
}) {
  const tokens = instanceIconStyles.sizes[size];
  const src =
    iconPath !== null && iconPath !== ""
      ? (assetSrc(iconPath) ?? grassFallback)
      : grassFallback;

  return (
    <span
      aria-hidden
      className={cn(instanceIconStyles.root, tokens.frame, className)}
    >
      <img
        src={src}
        alt=""
        className={instanceIconStyles.image}
        draggable={false}
      />
    </span>
  );
}
