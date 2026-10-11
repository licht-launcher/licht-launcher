import { useQuery } from "@tanstack/react-query";

import { useToastError } from "@/features/settings/hooks/use-toast-error";
import { getSettings, listInstances, listVersions } from "@/lib/commands";

export function useInstancesData() {
  const versions = useQuery({
    queryKey: ["versions"],
    queryFn: listVersions,
    retry: false,
  });

  const settings = useQuery({
    queryKey: ["settings"],
    queryFn: getSettings,
    retry: false,
  });

  const instances = useQuery({
    queryKey: ["instances"],
    queryFn: listInstances,
    retry: false,
  });

  useToastError(versions.error);
  useToastError(settings.error);
  useToastError(instances.error);

  const username = (settings.data?.settings.username ?? "Steve").trim();

  return {
    versions,
    settings,
    instances,
    username,
  };
}
