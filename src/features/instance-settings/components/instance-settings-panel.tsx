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
    <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden rounded-r-xl bg-muted/30">
      <ScrollArea
        className="min-h-0 flex-1"
        /* Radix top/bottom are inline; height:auto so h-full does not ignore bottom. */
        scrollbarStyle={{ top: "2.5rem", bottom: "0.75rem", height: "auto" }}
      >
        <div className="flex flex-col gap-4 px-6 pt-6 pr-12 pb-6">
          <div className="flex flex-col gap-1">
            <h2 className="text-base leading-7 font-medium">{meta?.label}</h2>
            <p className="text-sm text-muted-foreground">{meta?.description}</p>
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
