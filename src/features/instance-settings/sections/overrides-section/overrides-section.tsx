import { Card } from "@/components/ui/card";
import type { InstanceDraft } from "@/features/instance-settings/model/draft";
import { globalValues } from "@/features/instance-settings/model/global-values";
import { ArgumentsSection } from "@/features/instance-settings/sections/arguments-section";
import { JavaSection } from "@/features/instance-settings/sections/java-section";
import { MemorySection } from "@/features/instance-settings/sections/memory-section";
import { WindowSection } from "@/features/instance-settings/sections/window-section";
import type { SettingsSnapshot } from "@/lib/generated/SettingsSnapshot";

import { overridesSectionStyles } from "./overrides-section.styles";

export function OverridesSection({
  draft,
  snapshot,
  onChange,
}: {
  draft: InstanceDraft;
  snapshot: SettingsSnapshot;
  onChange: (patch: Partial<InstanceDraft>) => void;
}) {
  const global = globalValues(snapshot.settings);
  return (
    <div className={overridesSectionStyles.root}>
      <Card title="Game window">
        <WindowSection
          draft={draft}
          globalHint={global.window}
          onChange={onChange}
        />
      </Card>
      <Card title="Memory">
        <MemorySection
          draft={draft}
          snapshot={snapshot}
          globalHint={global.memory}
          onChange={onChange}
        />
      </Card>
      <Card title="Java">
        <JavaSection
          draft={draft}
          globalHint={global.java}
          onChange={onChange}
        />
        <ArgumentsSection
          draft={draft}
          globalHint={global.arguments}
          onChange={onChange}
        />
      </Card>
    </div>
  );
}
