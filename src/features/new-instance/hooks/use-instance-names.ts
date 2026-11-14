import { useQuery } from "@tanstack/react-query";

import { listInstances } from "@/api/commands";

/** Library instances; shares its cache with the instance list. */
export function useInstanceList() {
  return useQuery({
    queryKey: ["instances"],
    queryFn: listInstances,
    retry: false,
  });
}

/** Names already used by instances; shares its cache with the library list. */
export function useInstanceNames(): string[] {
  return (useInstanceList().data ?? []).map((instance) => instance.name);
}
