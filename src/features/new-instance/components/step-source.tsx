import { FileUpIcon, PackageIcon, WrenchIcon } from "lucide-react";

import { ChoiceCard } from "@/components/ui/choice-card";
import type { InstanceSource } from "@/features/new-instance/model/wizard";

export function StepSource({
  value,
  onChange,
}: {
  value: InstanceSource | null;
  onChange: (source: InstanceSource) => void;
}) {
  return (
    <div className="flex flex-col gap-4">
      <div>
        <h2 className="text-lg font-medium">How do you want to start?</h2>
        <p className="text-sm text-muted-foreground">
          Build a custom instance, browse a community pack, or import a file.
        </p>
      </div>
      <div className="grid gap-3 sm:grid-cols-2">
        <ChoiceCard
          title="From scratch"
          description="Choose a platform, name, and game version."
          icon={<WrenchIcon aria-hidden strokeWidth={2} />}
          iconTone="instance"
          selected={value === "scratch"}
          onSelect={() => {
            onChange("scratch");
          }}
        />
        <ChoiceCard
          title="From a modpack"
          description="Browse popular packs from Modrinth and CurseForge."
          icon={<PackageIcon aria-hidden strokeWidth={2} />}
          iconTone="warning"
          selected={value === "modpack"}
          onSelect={() => {
            onChange("modpack");
          }}
        />
        <ChoiceCard
          title="Import a pack file"
          description="Use a local .mrpack or CurseForge .zip."
          icon={<FileUpIcon aria-hidden strokeWidth={2} />}
          iconTone="file"
          selected={value === "file"}
          onSelect={() => {
            onChange("file");
          }}
        />
      </div>
    </div>
  );
}
