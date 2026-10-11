import { useEffect, useState } from "react";

import type { InstanceAction } from "@/features/instances/components/instance-action-dialog";
import type { SectionId } from "@/features/settings/model/sections";
import { onOpenSettingsRequest } from "@/lib/shell-events";

export function useInstanceDialogs() {
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsSection, setSettingsSection] = useState<SectionId>("account");
  const [configuringFolder, setConfiguringFolder] = useState<string | null>(
    null,
  );
  const [action, setAction] = useState<InstanceAction | null>(null);
  const [draftName, setDraftName] = useState("");

  useEffect(() => {
    return onOpenSettingsRequest((section) => {
      setConfiguringFolder(null);
      setSettingsSection(section);
      setSettingsOpen(true);
    });
  }, []);

  function clearAction() {
    setAction(null);
  }

  function openConfigure(folder: string) {
    setConfiguringFolder(folder);
  }

  function openGlobalSettings(section: SectionId = "account") {
    setConfiguringFolder(null);
    setSettingsSection(section);
    setSettingsOpen(true);
  }

  function openDuplicate(folder: string, name: string) {
    setDraftName("");
    setAction({ kind: "duplicate", folder, name });
  }

  function openDelete(folder: string, name: string) {
    setDraftName("");
    setAction({ kind: "delete", folder, name });
  }

  return {
    settingsOpen,
    setSettingsOpen,
    settingsSection,
    configuringFolder,
    setConfiguringFolder,
    action,
    draftName,
    setDraftName,
    clearAction,
    openConfigure,
    openGlobalSettings,
    openDuplicate,
    openDelete,
  };
}
