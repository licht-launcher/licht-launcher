import type { ModpackVersionItem } from "@/lib/generated/ModpackVersionItem";

/**
 * The version the page installs: the newest one, including betas and alphas.
 * `versions` is newest first.
 */
export function defaultVersionId(
  versions: readonly ModpackVersionItem[],
): string | null {
  return versions[0]?.id ?? null;
}
