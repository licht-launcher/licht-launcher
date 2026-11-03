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

export function PlatformPicker({
  value,
  onChange,
}: {
  value: PlatformId;
  onChange: (platform: PlatformId) => void;
}) {
  return (
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
  );
}
