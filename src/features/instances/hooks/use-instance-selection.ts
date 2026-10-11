import { useEffect, useState } from "react";

import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

export function useInstanceSelection(list: InstanceEntry[] | undefined) {
  const [selectedFolder, setSelectedFolder] = useState<string | null>(null);

  useEffect(() => {
    if (!list) {
      return;
    }
    if (list.length === 0) {
      setSelectedFolder(null);
      return;
    }
    if (
      selectedFolder === null ||
      !list.some((instance) => instance.folder === selectedFolder)
    ) {
      setSelectedFolder(list[0]?.folder ?? null);
    }
  }, [list, selectedFolder]);

  const selected =
    list?.find((instance) => instance.folder === selectedFolder) ?? null;

  return {
    selectedFolder,
    setSelectedFolder,
    selected,
  };
}
