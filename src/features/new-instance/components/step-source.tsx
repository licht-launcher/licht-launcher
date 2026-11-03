import { CheckIcon, FileUpIcon, PackageIcon, WrenchIcon } from "lucide-react";
import type { ReactNode } from "react";

import { ChoiceCard } from "@/components/ui/choice-card";
import type { InstanceSource } from "@/features/new-instance/model/wizard";
import { cn } from "@/lib/utils";

const tones = {
  cool: "text-accent-cool",
  warning: "text-warning",
  file: "text-accent-file",
} as const;

type Tone = keyof typeof tones;

/** Highlights a keyword with the card's tone instead of a heavier weight. */
function Key({ tone, children }: { tone: Tone; children: ReactNode }) {
  return <strong className={cn("font-medium", tones[tone])}>{children}</strong>;
}

function Code({ children }: { children: ReactNode }) {
  return (
    <code className="rounded bg-black/25 px-1 py-0.5 font-mono text-xs text-foreground">
      {children}
    </code>
  );
}

function Features({ tone, items }: { tone: Tone; items: string[] }) {
  return (
    <ul className="mt-1 flex flex-col gap-1.5 text-sm text-muted-foreground">
      {items.map((item) => (
        <li key={item} className="flex items-start gap-2">
          <CheckIcon
            aria-hidden
            className={cn("mt-0.5 size-3.5 shrink-0", tones[tone])}
            strokeWidth={2.5}
          />
          {item}
        </li>
      ))}
    </ul>
  );
}

function SelectedMark({ selected }: { selected: boolean }) {
  return (
    <span
      data-slot="selected-mark"
      data-selected={selected}
      aria-hidden={!selected}
      className={cn(
        "absolute top-3 right-3 flex size-6 items-center justify-center rounded-full bg-primary text-primary-foreground",
        "transition-[opacity,transform] duration-200 ease-out",
        selected ? "scale-100 opacity-100" : "scale-50 opacity-0",
      )}
    >
      <CheckIcon aria-hidden className="size-3.5" strokeWidth={3} />
      {selected ? <span className="sr-only">Selected</span> : null}
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
              Choose a <Key tone="cool">platform</Key>, a{" "}
              <Key tone="cool">name</Key> and a{" "}
              <Key tone="cool">game version</Key>.
            </>
          }
          icon={<WrenchIcon aria-hidden strokeWidth={2} />}
          iconTone="cool"
          selected={value === "scratch"}
          onSelect={() => {
            onChange("scratch");
          }}
        >
          <Features
            tone="cool"
            items={[
              "Vanilla, Fabric, NeoForge, Forge or Quilt",
              "Starts empty, add mods later",
            ]}
          />
          <SelectedMark selected={value === "scratch"} />
        </ChoiceCard>
        <ChoiceCard
          size="lg"
          className="relative"
          title="From a modpack"
          description={
            <>
              Browse popular packs from <Key tone="warning">Modrinth</Key> and{" "}
              <Key tone="warning">CurseForge</Key>.
            </>
          }
          icon={<PackageIcon aria-hidden strokeWidth={2} />}
          iconTone="warning"
          selected={value === "modpack"}
          onSelect={() => {
            onChange("modpack");
          }}
        >
          <Features
            tone="warning"
            items={[
              "Mods come already selected",
              "Choose any published pack version",
            ]}
          />
          <SelectedMark selected={value === "modpack"} />
        </ChoiceCard>
        <ChoiceCard
          size="lg"
          className="relative"
          title="Import a pack file"
          description={
            <>
              Use a local <Code>.mrpack</Code> or CurseForge <Code>.zip</Code>.
            </>
          }
          icon={<FileUpIcon aria-hidden strokeWidth={2} />}
          iconTone="file"
          selected={value === "file"}
          onSelect={() => {
            onChange("file");
          }}
        >
          <Features
            tone="file"
            items={[
              "The format is detected for you",
              "Your original file is left untouched",
            ]}
          />
          <SelectedMark selected={value === "file"} />
        </ChoiceCard>
      </div>
    </div>
  );
}
