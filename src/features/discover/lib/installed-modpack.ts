import type { InstanceEntry } from "@/lib/generated/InstanceEntry";
import type { ModpackSource } from "@/lib/generated/ModpackSource";

/** Whether the library already has an instance created from this store pack. */
export function hasInstalledModpack(
  instances: readonly InstanceEntry[],
  source: ModpackSource,
  projectId: string,
): boolean {
  return instances.some((instance) => {
    const origin = instance.installation?.source;
    return (
      origin?.type === "modpack" &&
      origin.source === source &&
      origin.projectId === projectId
    );
  });
}
