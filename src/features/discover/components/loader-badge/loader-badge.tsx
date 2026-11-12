import { Badge } from "@/components/ui/badge";
import { LOADER_LABELS } from "@/features/discover/lib/filter-options";
import { PlatformIcon } from "@/features/new-instance/assets/platform-icons";
import type { ModpackLoader } from "@/lib/generated/ModpackLoader";
import { cn } from "@/lib/utils";

import { loaderBadgeStyles } from "./loader-badge.styles";

/** A loader's official logo and name, in that loader's own color. */
export function LoaderBadge({ loader }: { loader: ModpackLoader }) {
  return (
    <Badge
      data-loader={loader}
      className={cn(loaderBadgeStyles.root, loaderBadgeStyles.tones[loader])}
    >
      <PlatformIcon platform={loader} className={loaderBadgeStyles.icon} />
      {LOADER_LABELS.get(loader)}
    </Badge>
  );
}
