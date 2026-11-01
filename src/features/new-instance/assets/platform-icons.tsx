import type { ReactNode } from "react";

import fabricUrl from "@/assets/platforms/fabric.png?no-inline";
import forgeUrl from "@/assets/platforms/forge.png?no-inline";
import neoforgeUrl from "@/assets/platforms/neoforge.png?no-inline";
import quiltUrl from "@/assets/platforms/quilt.svg?no-inline";
import vanillaUrl from "@/assets/platforms/vanilla.svg?no-inline";
import type { PlatformId } from "@/features/new-instance/model/create-state";
import { cn } from "@/lib/utils";

const PLATFORM_ICON_URL: Record<PlatformId, string> = {
  vanilla: vanillaUrl,
  fabric: fabricUrl,
  forge: forgeUrl,
  neoforge: neoforgeUrl,
  quilt: quiltUrl,
};

export function PlatformIcon({
  platform,
  className = "size-5",
}: {
  platform: PlatformId;
  className?: string;
}): ReactNode {
  return (
    <img
      src={PLATFORM_ICON_URL[platform]}
      alt=""
      className={cn("aspect-square object-contain", className)}
      draggable={false}
    />
  );
}

export function formatModpackSource(source: "modrinth" | "curseforge"): string {
  return source === "modrinth" ? "Modrinth" : "CurseForge";
}
