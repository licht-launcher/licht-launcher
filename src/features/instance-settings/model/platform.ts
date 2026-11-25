import type { InstancePlatform } from "@/lib/generated/InstancePlatform";
import type { LoaderPlatform } from "@/lib/generated/LoaderPlatform";

const PLATFORM_LABELS: Record<LoaderPlatform, string> = {
  vanilla: "Vanilla",
  fabric: "Fabric",
  quilt: "Quilt",
  forge: "Forge",
  neoForge: "NeoForge",
};

/**
 * A known platform never changes for a version id; an unknown one is asked again,
 * since its profile may be installed after the first look.
 */
export function platformStaleTime(
  platform: InstancePlatform | null | undefined,
) {
  return platform === null ? 0 : Number.POSITIVE_INFINITY;
}

/** The loader and its build, e.g. `Fabric 0.19.5`, or `Unknown` without a profile. */
export function formatPlatform(platform: InstancePlatform | null): string {
  if (platform === null) return "Unknown";
  const label = PLATFORM_LABELS[platform.platform];
  return platform.loaderVersion ? `${label} ${platform.loaderVersion}` : label;
}
