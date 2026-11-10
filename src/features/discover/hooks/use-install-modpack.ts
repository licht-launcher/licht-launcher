import { useMutation } from "@tanstack/react-query";

import { createInstanceFromModpack } from "@/api/commands";
import { useFinishCreation } from "@/features/new-instance/hooks/use-finish-creation";
import type { ModpackDetails } from "@/lib/generated/ModpackDetails";

/** Creates an instance from one pack version, then opens it in the library. */
export function useInstallModpack(modpack: ModpackDetails) {
  const { finish, reportFailure } = useFinishCreation();

  return useMutation({
    mutationFn: (input: { name: string; versionId: string }) =>
      createInstanceFromModpack({
        name: input.name,
        source: modpack.source,
        projectId: modpack.id,
        versionId: input.versionId,
        iconUrl: modpack.iconUrl,
      }),
    onSuccess: finish,
    onError: reportFailure,
  });
}
