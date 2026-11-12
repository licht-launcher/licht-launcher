import type { ModpackVersionItem } from "@/lib/generated/ModpackVersionItem";

/**
 * The version an install form starts with: the newest release, or the newest
 * version when a pack has only betas and alphas. `versions` is newest first.
 */
export function defaultVersionId(
  versions: readonly ModpackVersionItem[],
): string | null {
  const release = versions.find((version) => version.versionType === "release");
  return (release ?? versions[0])?.id ?? null;
}
