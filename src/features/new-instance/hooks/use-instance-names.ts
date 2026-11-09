import { useQuery } from "@tanstack/react-query";

import { listInstances } from "@/api/commands";

/** Names already used by instances; shares its cache with the library list. */
export function useInstanceNames(): string[] {
  const instances = useQuery({
    queryKey: ["instances"],
    queryFn: listInstances,
    retry: false,
  });
  return (instances.data ?? []).map((instance) => instance.name);
}
