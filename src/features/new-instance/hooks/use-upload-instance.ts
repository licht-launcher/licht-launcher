import { useMutation } from "@tanstack/react-query";

import { useFinishCreation } from "@/features/new-instance/hooks/use-finish-creation";
import { useInstanceNames } from "@/features/new-instance/hooks/use-instance-names";
import {
  uniqueName,
  uploadInstanceName,
} from "@/features/new-instance/model/create-state";
import {
  browseModpackFile,
  createInstanceFromModpackFile,
} from "@/lib/commands";

/**
 * Creates an instance from a pack file chosen in the system picker, named
 * after the file. Cancelling the picker does nothing.
 */
export function useUploadInstance() {
  const takenNames = useInstanceNames();
  const { finish, reportFailure } = useFinishCreation();

  const upload = useMutation({
    mutationFn: async () => {
      const path = await browseModpackFile();
      if (path === null) {
        return null;
      }
      return createInstanceFromModpackFile({
        name: uniqueName(uploadInstanceName(path), takenNames),
        path,
      });
    },
    onSuccess: async (entry) => {
      if (entry !== null) {
        await finish(entry);
      }
    },
    onError: reportFailure,
  });

  return { upload };
}
