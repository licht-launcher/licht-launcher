import { BlocksIcon, ImageIcon, SparklesIcon } from "lucide-react";
import type { InstanceContentKind } from "@/lib/generated/InstanceContentKind";
import { cn } from "@/lib/utils";

import { contentKindIconStyles as styles } from "./content-kind-icon.styles";

const ICONS = {
  mods: BlocksIcon,
  resourcePacks: ImageIcon,
  shaderPacks: SparklesIcon,
} as const;

/** The tinted icon that stands for a kind of content wherever content is listed. */
export function ContentKindIcon({
  kind,
  size = "default",
}: {
  kind: InstanceContentKind;
  size?: keyof typeof styles.sizes;
}) {
  const Icon = ICONS[kind];
  return (
    <span
      aria-hidden
      data-kind={kind}
      className={cn(
        styles.well,
        styles.sizes[size].well,
        styles.kinds[kind].well,
      )}
    >
      <Icon className={cn(styles.sizes[size].icon, styles.kinds[kind].icon)} />
    </span>
  );
}
