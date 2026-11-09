import { ChoiceCard } from "@/components/ui/choice-card";
import { PlatformIcon } from "@/features/new-instance/assets/platform-icons";
import {
  PLATFORMS,
  type PlatformId,
} from "@/features/new-instance/model/create-state";

import { platformPickerStyles } from "./platform-picker.styles";

export function PlatformPicker({
  value,
  onChange,
}: {
  value: PlatformId;
  onChange: (platform: PlatformId) => void;
}) {
  return (
    <fieldset className={platformPickerStyles.root}>
      <legend className={platformPickerStyles.legend}>Platform</legend>
      {PLATFORMS.map((platform) => (
        <ChoiceCard
          key={platform.id}
          className={platformPickerStyles.card}
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
