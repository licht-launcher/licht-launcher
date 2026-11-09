import { useQuery } from "@tanstack/react-query";
import { getSettings, listInstances } from "@/api/commands";
import { useToastError } from "@/features/settings/hooks/use-toast-error";

export function useInstancesData() {
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

  useToastError(settings.error);
  useToastError(instances.error);

  const username = (settings.data?.settings.username ?? "Steve").trim();

  return {
    settings,
    instances,
    username,
  };
}
