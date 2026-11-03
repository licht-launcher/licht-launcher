import { FileUpIcon, PackageIcon, WrenchIcon } from "lucide-react";

import { ChoiceCard } from "@/components/ui/choice-card";

export function SourcePicker({
  busy,
  onScratch,
  onUpload,
}: {
  busy: boolean;
  onScratch: () => void;
  onUpload: () => void;
}) {
  return (
    <div className="grid gap-3 sm:grid-cols-3">
      <ChoiceCard
        title="From scratch"
        description="Pick a platform and a game version."
        icon={<WrenchIcon aria-hidden strokeWidth={2} />}
        iconTone="cool"
        selected={false}
        disabled={busy}
        onSelect={onScratch}
      />
      <ChoiceCard
        title="From a modpack"
        description="Browse packs from Modrinth and CurseForge."
        icon={<PackageIcon aria-hidden strokeWidth={2} />}
        iconTone="warning"
        selected={false}
        disabled
        onSelect={() => {}}
      >
        <span className="rounded-full border border-warning/40 px-2 py-0.5 text-xs text-warning">
          Soon
        </span>
      </ChoiceCard>
      <ChoiceCard
        title="Upload a pack file"
        description="Use a local .mrpack or CurseForge .zip."
        icon={<FileUpIcon aria-hidden strokeWidth={2} />}
        iconTone="file"
        selected={false}
        disabled={busy}
        onSelect={onUpload}
      />
    </div>
  );
}
