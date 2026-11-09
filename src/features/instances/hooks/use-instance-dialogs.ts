import { useEffect, useState } from "react";
import { onOpenSettingsRequest } from "@/api/shell-events";
import type { InstanceAction } from "@/features/instances/components/instance-action-dialog";
import type { SectionId } from "@/features/settings/model/sections";

export function useInstanceDialogs() {
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [createOpen, setCreateOpen] = useState(false);
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

  function openCreate() {
    setCreateOpen(true);
  }

  function openConfigure(folder: string) {
    setConfiguringFolder(folder);
  }

  function openGlobalSettings(section: SectionId = "account") {
    setConfiguringFolder(null);
    setSettingsSection(section);
    setSettingsOpen(true);
  }

  function openRename(folder: string, name: string) {
    setDraftName(name);
    setAction({ kind: "rename", folder, name });
  }

  function openDelete(folder: string, name: string) {
    setDraftName("");
    setAction({ kind: "delete", folder, name });
  }

  return {
    settingsOpen,
    setSettingsOpen,
    createOpen,
    setCreateOpen,
    openCreate,
    settingsSection,
    configuringFolder,
    setConfiguringFolder,
    action,
    draftName,
    setDraftName,
    clearAction,
    openConfigure,
    openGlobalSettings,
    openRename,
    openDelete,
  };
}
