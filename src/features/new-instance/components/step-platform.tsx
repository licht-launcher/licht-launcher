import { ChoiceCard } from "@/components/ui/choice-card";
import { PlatformIcon } from "@/features/new-instance/assets/platform-icons";
import {
  PLATFORMS,
  type PlatformId,
} from "@/features/new-instance/model/create-state";

const PLATFORM_HINTS: Record<PlatformId, string> = {
  vanilla: "Official game, no mod loader.",
  fabric: "Lightweight mods, fast updates.",
  neoforge: "Modern Forge fork for recent versions.",
  forge: "Classic mod ecosystem.",
  quilt: "Fabric-compatible loader fork.",
};

export function StepPlatform({
  value,
  onChange,
}: {
  value: PlatformId | null;
  onChange: (platform: PlatformId) => void;
}) {
  return (
    <div className="flex flex-col gap-4">
      <div>
        <h2 className="text-lg font-medium">Platform</h2>
        <p className="text-sm text-muted-foreground">
          Vanilla is the official game. Loaders add mod support.
        </p>
      </div>
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
    </div>
  );
}
