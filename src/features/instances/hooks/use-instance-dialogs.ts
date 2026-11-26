import { useState } from "react";
import { toast } from "sonner";
import { openInstanceSettingsWindow } from "@/api/commands";
import { requestOpenSettings } from "@/api/shell-events";
import type { InstanceAction } from "@/features/instances/components/instance-action-dialog";
import type { SectionId } from "@/features/settings/model/sections";
import { friendlyError, messageOf } from "@/lib/errors";

export function useInstanceDialogs() {
  const [createOpen, setCreateOpen] = useState(false);
  const [action, setAction] = useState<InstanceAction | null>(null);
  const [draftName, setDraftName] = useState("");

  function clearAction() {
    setAction(null);
  }

  function clearActionFor(folder: string) {
    setAction((current) => (current?.folder === folder ? null : current));
  }

  function openCreate() {
    setCreateOpen(true);
  }

  function openConfigure(folder: string) {
    openInstanceSettingsWindow(folder).catch((error: unknown) => {
      toast.error(friendlyError(messageOf(error)));
    });
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
    action,
    draftName,
    setDraftName,
    clearAction,
    clearActionFor,
    openConfigure,
    openGlobalSettings,
    openRename,
    openDelete,
  };
}
