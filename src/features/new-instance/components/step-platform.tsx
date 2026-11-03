import { BoxIcon, TagIcon } from "lucide-react";
import type { ReactNode } from "react";

import { ChoiceCard } from "@/components/ui/choice-card";
import { Input } from "@/components/ui/input";
import { PlatformIcon } from "@/features/new-instance/assets/platform-icons";
import {
  PLATFORMS,
  type PlatformId,
} from "@/features/new-instance/model/wizard";

const PLATFORM_HINTS: Record<PlatformId, string> = {
  vanilla: "Official game, no mod loader.",
  fabric: "Lightweight mods, fast updates.",
  neoforge: "Modern Forge fork for recent versions.",
  forge: "Classic mod ecosystem.",
  quilt: "Fabric-compatible loader fork.",
};

function SectionHeader({
  icon,
  title,
  description,
}: {
  icon: ReactNode;
  title: string;
  description: string;
}) {
  return (
    <div className="flex items-center gap-3">
      <span className="flex size-10 shrink-0 items-center justify-center rounded-md bg-instance/20 text-instance">
        {icon}
      </span>
      <div>
        <h2 className="text-lg font-medium">{title}</h2>
        <p className="text-sm text-muted-foreground">{description}</p>
      </div>
    </div>
  );
}

export function StepPlatform({
  value,
  name,
  onChange,
  onNameChange,
}: {
  value: PlatformId | null;
  name: string;
  onChange: (platform: PlatformId) => void;
  onNameChange: (name: string) => void;
}) {
  return (
    <div className="flex flex-col gap-6">
      <section className="flex flex-col gap-3">
        <SectionHeader
          icon={<TagIcon aria-hidden className="size-5" strokeWidth={2} />}
          title="Name your instance"
          description="You can rename it later from the library."
        />
        <label
          className="flex max-w-md flex-col gap-1.5 text-sm font-semibold"
          htmlFor="instance-name"
        >
          Instance Name
          <Input
            id="instance-name"
            value={name}
            autoComplete="off"
            placeholder="Survival"
            onChange={(event) => {
              onNameChange(event.target.value);
            }}
          />
        </label>
      </section>
      <section className="flex flex-col gap-3 border-t border-border/40 pt-6">
        <SectionHeader
          icon={<BoxIcon aria-hidden className="size-5" strokeWidth={2} />}
          title="Choose a platform"
          description="Vanilla is the official game. Loaders add mod support."
        />
        <div className="grid gap-2 sm:grid-cols-2">
          {PLATFORMS.map((platform) => (
            <ChoiceCard
              key={platform.id}
              title={platform.label}
              description={PLATFORM_HINTS[platform.id]}
              icon={<PlatformIcon platform={platform.id} />}
              iconNeutral
              selected={value === platform.id}
              onSelect={() => {
                onChange(platform.id);
              }}
            />
          ))}
        </div>
      </section>
    </div>
  );
}
