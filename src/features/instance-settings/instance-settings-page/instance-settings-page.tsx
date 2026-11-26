import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import type { InstanceBaseInput } from "@/api/commands";
import { getSettings } from "@/api/commands";
import { Spinner } from "@/components/ui/spinner";
import { InstanceSettingsPanel } from "@/features/instance-settings/components/instance-settings-panel";
import { InstanceSettingsSidebar } from "@/features/instance-settings/components/instance-settings-sidebar";
import { useInstanceDraft } from "@/features/instance-settings/hooks/use-instance-draft";
import type { InstanceSectionId } from "@/features/instance-settings/model/sections";
import { modpackOrigin } from "@/features/instances/lib/modpack-content";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

import { instanceSettingsPageStyles } from "./instance-settings-page.styles";

export function InstanceSettingsPage({
  entry,
  busy,
  versionBusy,
  instanceActive = false,
  onOpenGlobalSettings,
  onRename,
  onChangeBase,
  onOpenFolder,
  onDuplicate,
  onDelete,
}: {
  entry: InstanceEntry | null;
  busy: boolean;
  versionBusy: boolean;
  /** The instance's game or a download for it is running. */
  instanceActive?: boolean;
  onOpenGlobalSettings: () => void;
  onRename: (name: string) => void;
  onChangeBase: (base: InstanceBaseInput) => void;
  onOpenFolder: () => void;
  onDuplicate: () => void;
  onDelete: () => void;
}) {
  const [section, setSection] = useState<InstanceSectionId>("general");
  const origin = modpackOrigin(entry?.installation?.source);
  const { draft, update } = useInstanceDraft(entry);
  const settings = useQuery({
    queryKey: ["settings"],
    queryFn: getSettings,
    retry: false,
  });

  return (
    <section
      aria-label="Instance settings"
      className={instanceSettingsPageStyles.root}
    >
      {entry ? (
        <InstanceSettingsSidebar
          name={entry.name}
          section={section}
          onSectionChange={setSection}
          onOpenGlobalSettings={onOpenGlobalSettings}
        />
      ) : null}
      {entry && draft && settings.data ? (
        <InstanceSettingsPanel
          section={section}
          draft={draft}
          snapshot={settings.data}
          instanceName={entry.name}
          folder={entry.folder}
          versionId={entry.versionId}
          preparing={
            entry.installation !== undefined &&
            entry.installation.state !== "ready"
          }
          modpack={
            origin !== null
              ? { origin, linked: entry.installation?.modpack ?? null }
              : null
          }
          busy={busy}
          versionBusy={versionBusy}
          instanceActive={instanceActive}
          onChange={update}
          onRename={onRename}
          onChangeBase={onChangeBase}
          onOpenFolder={onOpenFolder}
          onDuplicate={onDuplicate}
          onDelete={onDelete}
        />
      ) : (
        <div className={instanceSettingsPageStyles.loading}>
          <Spinner
            className={instanceSettingsPageStyles.spinner}
            aria-label="Loading instance settings"
          />
        </div>
      )}
    </section>
  );
}
