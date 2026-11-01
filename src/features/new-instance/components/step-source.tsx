import {
  CheckCircle2Icon,
  FileUpIcon,
  PackageIcon,
  WrenchIcon,
} from "lucide-react";
import type { ReactNode } from "react";

import { ChoiceCard } from "@/components/ui/choice-card";
import type { InstanceSource } from "@/features/new-instance/model/wizard";

function SelectedBadge({ selected }: { selected: boolean }) {
  return (
    <span
      className={
        selected
          ? "mt-auto flex items-center gap-1 text-xs font-medium text-instance"
          : "mt-auto flex items-center gap-1 text-xs text-transparent"
      }
      aria-hidden={!selected}
    >
      <CheckCircle2Icon aria-hidden className="size-3.5" />
      {selected ? "Selected" : ""}
    </span>
  );
}

function Hint({ children }: { children: ReactNode }) {
  return <span className="text-xs text-muted-foreground">{children}</span>;
}

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
      <div className="grid gap-3 sm:grid-cols-3">
        <ChoiceCard
          size="lg"
          title="From scratch"
          description="Choose a platform, name, and game version."
          icon={<WrenchIcon aria-hidden strokeWidth={2} />}
          iconTone="instance"
          selected={value === "scratch"}
          onSelect={() => {
            onChange("scratch");
          }}
        >
          <Hint>Vanilla, Fabric, NeoForge, Forge or Quilt.</Hint>
          <SelectedBadge selected={value === "scratch"} />
        </ChoiceCard>
        <ChoiceCard
          size="lg"
          title="From a modpack"
          description="Browse popular packs from Modrinth and CurseForge."
          icon={<PackageIcon aria-hidden strokeWidth={2} />}
          iconTone="warning"
          selected={value === "modpack"}
          onSelect={() => {
            onChange("modpack");
          }}
        >
          <Hint>Ready to play, with mods already chosen.</Hint>
          <SelectedBadge selected={value === "modpack"} />
        </ChoiceCard>
        <ChoiceCard
          size="lg"
          title="Import a pack file"
          description="Use a local .mrpack or CurseForge .zip."
          icon={<FileUpIcon aria-hidden strokeWidth={2} />}
          iconTone="file"
          selected={value === "file"}
          onSelect={() => {
            onChange("file");
          }}
        >
          <Hint>Works offline for the pack itself.</Hint>
          <SelectedBadge selected={value === "file"} />
        </ChoiceCard>
      </div>
    </div>
  );
}
