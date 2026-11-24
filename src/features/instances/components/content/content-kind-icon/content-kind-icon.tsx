import { BlocksIcon, ImageIcon, SparklesIcon } from "lucide-react";
import type { InstanceContentKind } from "@/lib/generated/InstanceContentKind";
import { cn } from "@/lib/utils";

import { contentKindIconStyles as styles } from "./content-kind-icon.styles";

const ICONS = {
  mods: BlocksIcon,
  resourcePacks: ImageIcon,
  shaderPacks: SparklesIcon,
} as const;

/**
 * The icon that stands for a kind of content. Lists tint it by kind; summaries
 * such as content counts use the muted tone.
 */
export function ContentKindIcon({
  kind,
  size = "default",
  tone = "accent",
}: {
  kind: InstanceContentKind;
  size?: keyof typeof styles.sizes;
  tone?: "accent" | "muted";
}) {
  const Icon = ICONS[kind];
  const tint = tone === "accent" ? styles.kinds[kind] : styles.muted;
  return (
    <span
      aria-hidden
      data-kind={kind}
      className={cn(styles.well, styles.sizes[size].well, tint.well)}
    >
      <Icon className={cn(styles.sizes[size].icon, tint.icon)} />
    </span>
  );
}
