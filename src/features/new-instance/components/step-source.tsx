import { CheckIcon, FileUpIcon, PackageIcon, WrenchIcon } from "lucide-react";
import type { ReactNode } from "react";

import { ChoiceCard } from "@/components/ui/choice-card";
import type { InstanceSource } from "@/features/new-instance/model/wizard";

function Key({ children }: { children: ReactNode }) {
  return <strong className="font-semibold text-foreground">{children}</strong>;
}

function SelectedMark({ selected }: { selected: boolean }) {
  if (!selected) {
    return null;
  }
  return (
    <span className="absolute top-3 right-3 flex size-6 items-center justify-center rounded-full bg-primary text-primary-foreground">
      <CheckIcon aria-hidden className="size-3.5" strokeWidth={3} />
      <span className="sr-only">Selected</span>
    </span>
  );
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
          className="relative"
          title="From scratch"
          description={
            <>
              Choose a <Key>platform</Key>, a <Key>name</Key> and a{" "}
              <Key>game version</Key>.
            </>
          }
          icon={<WrenchIcon aria-hidden strokeWidth={2} />}
          iconTone="cool"
          selected={value === "scratch"}
          onSelect={() => {
            onChange("scratch");
          }}
        >
          <SelectedMark selected={value === "scratch"} />
        </ChoiceCard>
        <ChoiceCard
          size="lg"
          className="relative"
          title="From a modpack"
          description={
            <>
              Browse popular packs from <Key>Modrinth</Key> and{" "}
              <Key>CurseForge</Key>.
            </>
          }
          icon={<PackageIcon aria-hidden strokeWidth={2} />}
          iconTone="warning"
          selected={value === "modpack"}
          onSelect={() => {
            onChange("modpack");
          }}
        >
          <SelectedMark selected={value === "modpack"} />
        </ChoiceCard>
        <ChoiceCard
          size="lg"
          className="relative"
          title="Import a pack file"
          description={
            <>
              Use a local <Key>.mrpack</Key> or CurseForge <Key>.zip</Key>.
            </>
          }
          icon={<FileUpIcon aria-hidden strokeWidth={2} />}
          iconTone="file"
          selected={value === "file"}
          onSelect={() => {
            onChange("file");
          }}
        >
          <SelectedMark selected={value === "file"} />
        </ChoiceCard>
      </div>
    </div>
  );
}
