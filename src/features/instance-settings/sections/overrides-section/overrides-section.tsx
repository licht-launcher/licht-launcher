import type { InstanceDraft } from "@/features/instance-settings/model/draft";
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
  return (
    <div className={overridesSectionStyles.root}>
      <section
        className={overridesSectionStyles.section}
        aria-labelledby="override-window-title"
      >
        <h3 id="override-window-title" className={overridesSectionStyles.title}>
          Window
        </h3>
        <WindowSection draft={draft} onChange={onChange} />
      </section>
      <section
        className={overridesSectionStyles.dividedSection}
        aria-labelledby="override-memory-title"
      >
        <h3 id="override-memory-title" className={overridesSectionStyles.title}>
          Memory
        </h3>
        <MemorySection draft={draft} snapshot={snapshot} onChange={onChange} />
      </section>
      <section
        className={overridesSectionStyles.dividedSection}
        aria-labelledby="override-java-title"
      >
        <h3 id="override-java-title" className={overridesSectionStyles.title}>
          Java
        </h3>
        <div className={overridesSectionStyles.javaFields}>
          <JavaSection draft={draft} onChange={onChange} />
          <ArgumentsSection draft={draft} onChange={onChange} />
        </div>
      </section>
    </div>
  );
}
