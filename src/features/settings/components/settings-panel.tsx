import { ScrollArea } from "@/components/ui/scroll-area";
import type { Draft } from "@/features/settings/model/draft";
import { SECTIONS, type SectionId } from "@/features/settings/model/sections";
import { AccountSection } from "@/features/settings/sections/account-section";
import { AppearanceSection } from "@/features/settings/sections/appearance-section";
import { DownloadsSection } from "@/features/settings/sections/downloads-section";
import { InstancesSection } from "@/features/settings/sections/instances-section";
import { JavaSection } from "@/features/settings/sections/java/java-section";
import type { SettingsSnapshot } from "@/lib/generated/SettingsSnapshot";

export function SettingsPanel({
  section,
  draft,
  snapshot,
  onChange,
}: {
  section: SectionId;
  draft: Draft;
  snapshot: SettingsSnapshot;
  onChange: (patch: Partial<Draft>) => void;
}) {
  const page = SECTIONS.find((item) => item.id === section) ?? SECTIONS[0];

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-muted/30">
      <ScrollArea
        className="min-h-0 flex-1"
        /* Radix top/bottom are inline; height:auto so h-full does not ignore bottom. */
        scrollbarStyle={{ top: "2.5rem", bottom: "0.75rem", height: "auto" }}
      >
        <div className="flex flex-col gap-4 px-6 pt-6 pr-12 pb-6">
          <div className="flex flex-col gap-1">
            <h2 className="text-lg font-medium">{page.label}</h2>
            <p className="text-sm text-muted-foreground">{page.description}</p>
          </div>
          {section === "account" ? (
            <AccountSection draft={draft} onChange={onChange} />
          ) : null}
          {section === "appearance" ? (
            <AppearanceSection draft={draft} onChange={onChange} />
          ) : null}
          {section === "instances" ? (
            <InstancesSection
              draft={draft}
              snapshot={snapshot}
              onChange={onChange}
            />
          ) : null}
          {section === "java" ? (
            <JavaSection draft={draft} onChange={onChange} />
          ) : null}
          {section === "downloads" ? (
            <DownloadsSection draft={draft} onChange={onChange} />
          ) : null}
        </div>
      </ScrollArea>
    </div>
  );
}
