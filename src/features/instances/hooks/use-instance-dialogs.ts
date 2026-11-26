import { useState } from "react";
import { requestOpenSettings } from "@/api/shell-events";
import type { InstanceAction } from "@/features/instances/components/instance-action-dialog";
import type { SectionId } from "@/features/settings/model/sections";

export function useInstanceDialogs() {
  const [createOpen, setCreateOpen] = useState(false);
  const [configuringFolder, setConfiguringFolder] = useState<string | null>(
    null,
  );
  const [action, setAction] = useState<InstanceAction | null>(null);
  const [draftName, setDraftName] = useState("");

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
    requestOpenSettings(section);
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
    createOpen,
    setCreateOpen,
    openCreate,
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
