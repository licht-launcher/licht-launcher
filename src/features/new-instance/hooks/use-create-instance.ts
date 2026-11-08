import { useMutation } from "@tanstack/react-query";
import { createInstanceWithPlatform } from "@/api/commands";
import { useFinishCreation } from "@/features/new-instance/hooks/use-finish-creation";
import {
  resolveInstanceName,
  type ScratchState,
} from "@/features/new-instance/model/create-state";

/** Creates an instance from the scratch form, then opens it and closes the popup. */
export function useCreateInstance({
  scratch,
  takenNames,
  onCreated,
}: {
  scratch: ScratchState;
  takenNames: readonly string[];
  onCreated: () => void;
}) {
  const { finish, reportFailure } = useFinishCreation();

  return useMutation({
    mutationFn: () =>
      createInstanceWithPlatform({
        name: resolveInstanceName(scratch, takenNames),
        platform: scratch.platform,
        gameVersion: scratch.gameVersion,
        loaderChannel: scratch.loaderChannel,
        loaderVersion:
          scratch.loaderChannel === "manual" ? scratch.loaderVersion : null,
      }),
    onSuccess: async (entry) => {
      await finish(entry);
      onCreated();
    },
    onError: reportFailure,
  });
}
