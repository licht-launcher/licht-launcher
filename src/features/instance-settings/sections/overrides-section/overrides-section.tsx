import type { InstanceDraft } from "@/features/instance-settings/model/draft";
import { ArgumentsSection } from "@/features/instance-settings/sections/arguments-section";
import { JavaSection } from "@/features/instance-settings/sections/java-section";
import { MemorySection } from "@/features/instance-settings/sections/memory-section";
import { WindowSection } from "@/features/instance-settings/sections/window-section";
import type { SettingsSnapshot } from "@/lib/generated/SettingsSnapshot";

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
    <div className="flex flex-col gap-8">
      <section
        className="flex flex-col gap-4"
        aria-labelledby="override-window-title"
      >
        <h3 id="override-window-title" className="text-sm font-medium">
          Window
        </h3>
        <WindowSection draft={draft} onChange={onChange} />
      </section>
      <section
        className="flex flex-col gap-4 border-t border-border pt-6"
        aria-labelledby="override-memory-title"
      >
        <h3 id="override-memory-title" className="text-sm font-medium">
          Memory
        </h3>
        <MemorySection draft={draft} snapshot={snapshot} onChange={onChange} />
      </section>
      <section
        className="flex flex-col gap-4 border-t border-border pt-6"
        aria-labelledby="override-java-title"
      >
        <h3 id="override-java-title" className="text-sm font-medium">
          Java
        </h3>
        <div className="flex flex-col gap-6">
          <JavaSection draft={draft} onChange={onChange} />
          <ArgumentsSection draft={draft} onChange={onChange} />
        </div>
      </section>
    </div>
  );
}
