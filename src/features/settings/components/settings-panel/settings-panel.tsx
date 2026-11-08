import { PageTransition } from "@/components/ui/page-transition";
import { ScrollArea } from "@/components/ui/scroll-area";
import type { Draft } from "@/features/settings/model/draft";
import { SECTIONS, type SectionId } from "@/features/settings/model/sections";
import { AccountSection } from "@/features/settings/sections/account-section";
import { AppearanceSection } from "@/features/settings/sections/appearance-section";
import { BehaviorSection } from "@/features/settings/sections/behavior-section";
import { DevelopersSection } from "@/features/settings/sections/developers-section";
import { DownloadsSection } from "@/features/settings/sections/downloads-section";
import { InstancesSection } from "@/features/settings/sections/instances-section";
import { JavaSection } from "@/features/settings/sections/java/java-section";
import { UpdaterSection } from "@/features/settings/sections/updater-section";
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
    <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
      <ScrollArea
        className="min-h-0 flex-1"
        /* Radix top/bottom are inline; height:auto so h-full does not ignore bottom. */
        scrollbarStyle={{ top: "2.5rem", bottom: "0.75rem", height: "auto" }}
      >
        <PageTransition animationKey={section}>
          <div className="flex flex-col gap-6 px-8 pt-8 pr-12 pb-8">
            <div className="flex flex-col gap-0.5">
              <h2 className="text-lg font-medium">{page.label}</h2>
              <p className="text-sm leading-snug text-muted-foreground">
                {page.description}
              </p>
            </div>
            {section === "account" ? (
              <div className="flex flex-col gap-8">
                <AccountSection />
                <AppearanceSection draft={draft} onChange={onChange} />
                <BehaviorSection draft={draft} onChange={onChange} />
              </div>
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
            {section === "developers" ? (
              <DevelopersSection draft={draft} onChange={onChange} />
            ) : null}
            {section === "updater" ? (
              <UpdaterSection draft={draft} onChange={onChange} />
            ) : null}
          </div>
        </PageTransition>
      </ScrollArea>
    </div>
  );
}
