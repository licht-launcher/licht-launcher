import { ChoiceCard } from "@/components/ui/choice-card";
import { PlatformIcon } from "@/features/new-instance/assets/platform-icons";
import {
  PLATFORMS,
  type PlatformId,
} from "@/features/new-instance/model/create-state";

export function PlatformPicker({
  value,
  onChange,
}: {
  value: PlatformId;
  onChange: (platform: PlatformId) => void;
}) {
  return (
    <fieldset className="flex min-w-0 flex-wrap gap-2">
      <legend className="sr-only">Platform</legend>
      {PLATFORMS.map((platform) => (
        <ChoiceCard
          key={platform.id}
          className="w-auto"
          title={platform.label}
          icon={<PlatformIcon platform={platform.id} />}
          iconNeutral
          selected={value === platform.id}
          onSelect={() => {
            onChange(platform.id);
          }}
        />
      ))}
    </fieldset>
  );
}
