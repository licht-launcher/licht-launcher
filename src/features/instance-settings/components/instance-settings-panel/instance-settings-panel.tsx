import { ScrollArea } from "@/components/ui/scroll-area";
import type { InstanceDraft } from "@/features/instance-settings/model/draft";
import {
  INSTANCE_SECTIONS,
  type InstanceSectionId,
} from "@/features/instance-settings/model/sections";
import { GeneralSection } from "@/features/instance-settings/sections/general-section";
import { InstanceBaseSection } from "@/features/instance-settings/sections/instance-base-section";
import { OverridesSection } from "@/features/instance-settings/sections/overrides-section";
import type { SettingsSnapshot } from "@/lib/generated/SettingsSnapshot";

import { instanceSettingsPanelStyles } from "./instance-settings-panel.styles";

export function InstanceSettingsPanel({
  section,
  draft,
  snapshot,
  instanceName,
  folder,
  versionId,
  busy,
  versionBusy,
  preparing = false,
  onChange,
  onRename,
  onChangeVersion,
  onOpenFolder,
  onDuplicate,
  onDelete,
}: {
  section: InstanceSectionId;
  draft: InstanceDraft;
  snapshot: SettingsSnapshot;
  instanceName: string;
  folder: string;
  versionId: string | null;
  preparing?: boolean;
  busy: boolean;
  versionBusy: boolean;
  onChange: (patch: Partial<InstanceDraft>) => void;
  onRename: (name: string) => void;
  onChangeVersion: (versionId: string, installed: boolean) => void;
  onOpenFolder: () => void;
  onDuplicate: () => void;
  onDelete: () => void;
}) {
  const meta = INSTANCE_SECTIONS.find((item) => item.id === section);
  return (
    <div className={instanceSettingsPanelStyles.root}>
      <ScrollArea
        className={instanceSettingsPanelStyles.scrollArea}
        /* Radix top/bottom are inline; height:auto so h-full does not ignore bottom. */
        scrollbarStyle={{ top: "2.5rem", bottom: "0.75rem", height: "auto" }}
      >
        <div className={instanceSettingsPanelStyles.content}>
          <div className={instanceSettingsPanelStyles.header}>
            <h2 className={instanceSettingsPanelStyles.title}>{meta?.label}</h2>
            <p className={instanceSettingsPanelStyles.description}>
              {meta?.description}
            </p>
          </div>
          {section === "general" ? (
            <GeneralSection
              name={instanceName}
              busy={busy}
              onRename={onRename}
              onOpenFolder={onOpenFolder}
              onDuplicate={onDuplicate}
              onDelete={onDelete}
            />
          ) : null}
          {section === "base" ? (
            <InstanceBaseSection
              folder={folder}
              versionId={versionId}
              preparing={preparing}
              busy={busy}
              versionBusy={versionBusy}
              onChangeVersion={onChangeVersion}
            />
          ) : null}
          {section === "overrides" ? (
            <OverridesSection
              draft={draft}
              snapshot={snapshot}
              onChange={onChange}
            />
          ) : null}
        </div>
      </ScrollArea>
    </div>
  );
}
