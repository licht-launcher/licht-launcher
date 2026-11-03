import { useQuery } from "@tanstack/react-query";

import { listInstances } from "@/lib/commands";

/**
 * Names and folders already used by instances, which a new instance has to
 * avoid; shares its cache with the library list.
 */
export function useInstanceNames(): string[] {
  const instances = useQuery({
    queryKey: ["instances"],
    queryFn: listInstances,
    retry: false,
  });
  return (instances.data ?? []).flatMap((instance) => [
    instance.name,
    instance.folder,
  ]);
}
