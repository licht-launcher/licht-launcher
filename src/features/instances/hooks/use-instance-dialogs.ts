import { useEffect, useState } from "react";

import type { InstanceAction } from "@/features/instances/components/instance-action-dialog";
import { createVersionChoices } from "@/features/instances/model/create-versions";
import type { SectionId } from "@/features/settings/model/sections";
import type { VersionSummary } from "@/lib/generated/VersionSummary";
import { onOpenSettingsRequest } from "@/lib/shell-events";

export function useInstanceDialogs() {
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsSection, setSettingsSection] = useState<SectionId>("account");
  const [configuringFolder, setConfiguringFolder] = useState<string | null>(
    null,
  );
  const [action, setAction] = useState<InstanceAction | null>(null);
  const [draftName, setDraftName] = useState("");
  const [draftVersion, setDraftVersion] = useState("");
  const [includeSnapshots, setIncludeSnapshots] = useState(false);

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

  function openCreate(versions: VersionSummary[]) {
    setDraftName("");
    setIncludeSnapshots(false);
    const choices = createVersionChoices(versions, false);
    setDraftVersion(choices[0]?.id ?? "");
    setAction({ kind: "create" });
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
    draftVersion,
    setDraftVersion,
    includeSnapshots,
    setIncludeSnapshots,
    clearAction,
    openCreate,
    openConfigure,
    openGlobalSettings,
    openDuplicate,
    openDelete,
  };
}
